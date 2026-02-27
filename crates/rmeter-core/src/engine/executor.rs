use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio::task::JoinSet;
use tokio::time::{interval, sleep};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::engine::aggregator::StreamingAggregator;
use crate::engine::virtual_user::run_virtual_user;
use crate::engine::EngineStatus;
use crate::error::RmeterError;
use crate::plan::model::TestPlan;
use crate::results::{RequestResultEvent, TestSummary};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// CSV Data Set — runtime representation of CSV data sources
// ---------------------------------------------------------------------------

/// A single CSV source loaded into memory with an atomic row counter.
struct CsvSourceRuntime {
    columns: Vec<String>,
    rows: Vec<Vec<String>>,
    counter: std::sync::atomic::AtomicUsize,
    recycle: bool,
}

/// Combined runtime data from all CSV data sources in a plan.
/// Each iteration, virtual users call `next_row()` to get the next set of
/// CSV-derived variables merged into a HashMap.
pub struct CsvDataSet {
    sources: Vec<CsvSourceRuntime>,
}

impl CsvDataSet {
    /// Build a runtime data set from the plan's CSV data sources.
    pub fn from_sources(sources: &[crate::plan::model::CsvDataSource]) -> Self {
        Self {
            sources: sources
                .iter()
                .map(|s| CsvSourceRuntime {
                    columns: s.columns.clone(),
                    rows: s.rows.clone(),
                    counter: std::sync::atomic::AtomicUsize::new(0),
                    recycle: s.recycle,
                })
                .collect(),
        }
    }

    /// Return `true` if there are no CSV data sources.
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    /// Atomically grab the next row from each CSV source and merge the
    /// column→value pairs into a single HashMap. Sources that are exhausted
    /// (and have `recycle: false`) are skipped.
    pub fn next_row(&self) -> HashMap<String, String> {
        let mut vars = HashMap::new();
        for src in &self.sources {
            if src.rows.is_empty() {
                continue;
            }
            let idx = src.counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let row_idx = if src.recycle {
                idx % src.rows.len()
            } else if idx < src.rows.len() {
                idx
            } else {
                continue; // exhausted
            };
            let row = &src.rows[row_idx];
            for (col, val) in src.columns.iter().zip(row.iter()) {
                vars.insert(col.clone(), val.clone());
            }
        }
        vars
    }
}

/// An event emitted by the engine during test execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EngineEvent {
    /// A single HTTP request completed.
    RequestResult(RequestResultEvent),

    /// Periodic progress snapshot (~every 500 ms).
    Progress {
        completed_requests: u64,
        total_errors: u64,
        active_threads: u32,
        elapsed_ms: u64,
        current_rps: f64,
        mean_ms: f64,
        p95_ms: u64,
        min_ms: u64,
        max_ms: u64,
    },

    /// Engine lifecycle status changed.
    StatusChange { status: EngineStatus },

    /// Test run completed; final summary is attached.
    Complete { summary: TestSummary },
}

/// A handle to a running test that allows callers to inspect status and stop
/// execution.
pub struct EngineHandle {
    /// Cancel token — drop this or call `.cancel()` to trigger graceful stop.
    pub cancel_token: CancellationToken,
    /// Current engine lifecycle state.
    pub status: Arc<RwLock<EngineStatus>>,
    /// The shared aggregator — callers may read it for live stats.
    pub aggregator: Arc<RwLock<StreamingAggregator>>,
}

/// Configuration passed to [`run_test`].
pub struct EngineConfig {
    /// The test plan to execute.
    pub plan: TestPlan,
    /// Channel sender for engine events.
    pub result_tx: mpsc::Sender<EngineEvent>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Start executing a test plan asynchronously.
///
/// Returns an [`EngineHandle`] immediately; the engine runs in a background
/// Tokio task.
pub async fn run_test(config: EngineConfig) -> Result<EngineHandle, RmeterError> {
    let cancel_token = CancellationToken::new();
    let status = Arc::new(RwLock::new(EngineStatus::Running));
    let aggregator = Arc::new(RwLock::new(StreamingAggregator::new()));

    let handle = EngineHandle {
        cancel_token: cancel_token.clone(),
        status: status.clone(),
        aggregator: aggregator.clone(),
    };

    // Validate: at least one thread group must be enabled.
    let enabled_groups: Vec<_> = config
        .plan
        .thread_groups
        .iter()
        .filter(|tg| tg.enabled)
        .cloned()
        .collect();

    if enabled_groups.is_empty() {
        return Err(RmeterError::Validation(
            "Test plan has no enabled thread groups".to_string(),
        ));
    }

    // Emit initial Running status.
    let _ = config
        .result_tx
        .send(EngineEvent::StatusChange {
            status: EngineStatus::Running,
        })
        .await;

    let plan_id = config.plan.id;
    let plan_name = config.plan.name.clone();
    let plan_variables = config.plan.variables.clone();
    let csv_data_sources = config.plan.csv_data_sources.clone();

    // Spawn the main engine orchestrator.
    tokio::spawn(async move {
        execute_plan(
            plan_id,
            plan_name,
            enabled_groups,
            plan_variables,
            csv_data_sources,
            config.result_tx,
            cancel_token,
            status,
            aggregator,
        )
        .await;
    });

    Ok(handle)
}

// ---------------------------------------------------------------------------
// Internal implementation
// ---------------------------------------------------------------------------

/// Top-level plan executor. Runs all thread groups, aggregates results, and
/// emits a final [`EngineEvent::Complete`] when done.
#[allow(clippy::too_many_arguments)]
async fn execute_plan(
    plan_id: Uuid,
    plan_name: String,
    thread_groups: Vec<crate::plan::model::ThreadGroup>,
    plan_variables: Vec<crate::plan::model::Variable>,
    csv_data_sources: Vec<crate::plan::model::CsvDataSource>,
    result_tx: mpsc::Sender<EngineEvent>,
    cancel_token: CancellationToken,
    status: Arc<RwLock<EngineStatus>>,
    aggregator: Arc<RwLock<StreamingAggregator>>,
) {
    // Internal channel for collecting RequestResultEvents from virtual users.
    // The channel is intentionally unbounded to avoid blocking virtual user
    // tasks; the aggregation loop drains it.
    let (vu_tx, mut vu_rx) = mpsc::channel::<RequestResultEvent>(4096);

    // Build the initial variable map from the plan's variable definitions.
    // All thread groups share a single Arc<Mutex<_>> so that extractors in one
    // thread group can produce values consumed by another.
    let plan_vars: HashMap<String, String> = plan_variables
        .into_iter()
        .map(|v| (v.name, v.value))
        .collect();
    let shared_variables: Arc<Mutex<HashMap<String, String>>> =
        Arc::new(Mutex::new(plan_vars));

    // Build the shared CSV data set from all CSV data sources.
    let csv_data_set: Arc<CsvDataSet> = Arc::new(CsvDataSet::from_sources(&csv_data_sources));

    // Build a shared reqwest::Client that all virtual users will reuse (connection pool).
    let client = match reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .pool_max_idle_per_host(100)
        .pool_idle_timeout(Duration::from_secs(90))
        .user_agent(format!("rmeter/{}", env!("CARGO_PKG_VERSION")))
        .gzip(true)
        .brotli(true)
        .build()
    {
        Ok(c) => Arc::new(c),
        Err(e) => {
            emit_error_status(&result_tx, &status, format!("Failed to build HTTP client: {e}"))
                .await;
            return;
        }
    };

    // Keep track of total spawned virtual users so we can report active_threads.
    let active_threads = Arc::new(std::sync::atomic::AtomicU32::new(0));

    // Spawn all thread groups.
    let mut group_join_set: JoinSet<()> = JoinSet::new();

    for tg in thread_groups {
        let tg_name = tg.name.clone();
        let num_threads = tg.num_threads;
        let ramp_up_seconds = tg.ramp_up_seconds;
        let loop_count = tg.loop_count.clone();
        let requests = tg.requests.clone();
        let client = Arc::clone(&client);
        let vu_tx = vu_tx.clone();
        let cancel = cancel_token.clone();
        let active = Arc::clone(&active_threads);
        let variables = Arc::clone(&shared_variables);
        let csv = Arc::clone(&csv_data_set);

        group_join_set.spawn(async move {
            run_thread_group(
                plan_id,
                tg_name,
                num_threads,
                ramp_up_seconds,
                loop_count,
                requests,
                client,
                vu_tx,
                cancel,
                active,
                variables,
                csv,
            )
            .await;
        });
    }

    // Drop the original vu_tx so the channel closes when all thread-group tasks
    // drop their clones (i.e., when all virtual users finish).
    drop(vu_tx);

    // Progress reporter — emits periodic progress events every 500 ms.
    let agg_for_reporter = Arc::clone(&aggregator);
    let tx_for_reporter = result_tx.clone();
    let active_for_reporter = Arc::clone(&active_threads);
    let cancel_for_reporter = cancel_token.clone();
    let progress_task = tokio::spawn(async move {
        let mut ticker = interval(Duration::from_millis(500));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    let snap = agg_for_reporter.read().await.snapshot();
                    let _ = tx_for_reporter
                        .send(EngineEvent::Progress {
                            completed_requests: snap.total_requests,
                            total_errors: snap.total_errors,
                            active_threads: active_for_reporter
                                .load(std::sync::atomic::Ordering::Relaxed),
                            elapsed_ms: snap.elapsed_ms,
                            current_rps: snap.current_rps,
                            mean_ms: snap.mean_ms,
                            p95_ms: snap.p95_ms,
                            min_ms: snap.min_ms,
                            max_ms: snap.max_ms,
                        })
                        .await;
                }
                _ = cancel_for_reporter.cancelled() => break,
            }
        }
    });

    // Aggregation loop — drains the vu_rx channel and records each result.
    while let Some(event) = vu_rx.recv().await {
        let success = event.error.is_none();
        {
            let mut agg = aggregator.write().await;
            agg.record(event.elapsed_ms, success, event.size_bytes);
        }
        // Forward the raw result to external consumers.
        let _ = result_tx.send(EngineEvent::RequestResult(event)).await;
    }

    // All virtual users have finished (or been cancelled) — clean up.
    progress_task.abort();
    // Wait for all thread groups to fully exit.
    while group_join_set.join_next().await.is_some() {}

    // Both normal completion and graceful cancellation produce the same status.
    let final_status = EngineStatus::Completed;

    // Emit status change.
    {
        let mut s = status.write().await;
        *s = final_status.clone();
    }
    let _ = result_tx
        .send(EngineEvent::StatusChange {
            status: final_status,
        })
        .await;

    // Build and emit the final summary.
    let summary = aggregator
        .read()
        .await
        .summary(plan_id, plan_name);
    let _ = result_tx
        .send(EngineEvent::Complete { summary })
        .await;
}

/// Manages a single [`ThreadGroup`]: spawns `num_threads` virtual users with
/// configurable ramp-up pacing.
#[allow(clippy::too_many_arguments)]
async fn run_thread_group(
    plan_id: Uuid,
    tg_name: String,
    num_threads: u32,
    ramp_up_seconds: u32,
    loop_count: crate::plan::model::LoopCount,
    requests: Vec<crate::plan::model::HttpRequest>,
    client: Arc<reqwest::Client>,
    vu_tx: mpsc::Sender<RequestResultEvent>,
    cancel: CancellationToken,
    active_threads: Arc<std::sync::atomic::AtomicU32>,
    variables: Arc<Mutex<HashMap<String, String>>>,
    csv_data_set: Arc<CsvDataSet>,
) {
    if num_threads == 0 {
        return;
    }

    // Calculate ramp-up delay between thread starts.
    let ramp_delay = if ramp_up_seconds > 0 && num_threads > 1 {
        Duration::from_millis(
            (ramp_up_seconds as u64 * 1000) / (num_threads as u64 - 1).max(1),
        )
    } else {
        Duration::ZERO
    };

    let mut vu_join_set: JoinSet<()> = JoinSet::new();

    for user_id in 0..num_threads {
        if cancel.is_cancelled() {
            break;
        }

        // Stagger thread starts over the ramp-up period.
        if user_id > 0 && !ramp_delay.is_zero() {
            tokio::select! {
                _ = sleep(ramp_delay) => {}
                _ = cancel.cancelled() => break,
            }
        }

        let requests_clone = requests
            .iter()
            .filter(|r| r.enabled)
            .cloned()
            .collect::<Vec<_>>();

        if requests_clone.is_empty() {
            continue;
        }

        let client_clone = Arc::clone(&client);
        let vu_tx_clone = vu_tx.clone();
        let cancel_clone = cancel.clone();
        let tg_name_clone = tg_name.clone();
        let loop_count_clone = loop_count.clone();
        let active_clone = Arc::clone(&active_threads);
        let variables_clone = Arc::clone(&variables);
        let csv_clone = Arc::clone(&csv_data_set);

        active_threads.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        vu_join_set.spawn(async move {
            run_virtual_user(
                user_id,
                requests_clone,
                client_clone,
                cancel_clone,
                vu_tx_clone,
                plan_id,
                tg_name_clone,
                loop_count_clone,
                variables_clone,
                csv_clone,
            )
            .await;
            active_clone.fetch_sub(1, std::sync::atomic::Ordering::Relaxed);
        });
    }

    // Wait for all virtual users to finish.
    while vu_join_set.join_next().await.is_some() {}
}

// ---------------------------------------------------------------------------
// Error helpers
// ---------------------------------------------------------------------------

async fn emit_error_status(
    tx: &mpsc::Sender<EngineEvent>,
    status: &Arc<RwLock<EngineStatus>>,
    message: String,
) {
    tracing::error!("Engine error: {message}");
    {
        let mut s = status.write().await;
        *s = EngineStatus::Error;
    }
    let _ = tx
        .send(EngineEvent::StatusChange {
            status: EngineStatus::Error,
        })
        .await;
}
