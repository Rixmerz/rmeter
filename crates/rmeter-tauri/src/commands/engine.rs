use std::sync::Mutex;

use rmeter_core::engine::aggregator::{AggregatorSnapshot, TimeBucketEntry};
use rmeter_core::engine::executor::{EngineConfig, EngineEvent, run_test};
use rmeter_core::engine::{EngineHandle, EngineStatus, StreamingAggregator};
use rmeter_core::error::RmeterError;
use rmeter_core::plan::{PlanManager, validate_plan};
use rmeter_core::results::{RequestResultEvent, ResultStore, TestRunResult};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;

use std::sync::Arc;

// ---------------------------------------------------------------------------
// Managed state
// ---------------------------------------------------------------------------

/// Tauri-managed state for the running engine instance.
///
/// Wrapped in `Mutex` because Tauri's `State` requires `Send + Sync` and we
/// need exclusive access when starting/stopping the engine.
pub struct EngineState {
    pub handle: Option<EngineHandle>,
    pub aggregator: Option<Arc<RwLock<StreamingAggregator>>>,
}

impl EngineState {
    pub fn new() -> Self {
        Self {
            handle: None,
            aggregator: None,
        }
    }
}

impl Default for EngineState {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Tauri event payloads
// ---------------------------------------------------------------------------

/// Serializable payload emitted as "test-progress" Tauri event.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
struct ProgressPayload {
    completed_requests: u64,
    total_errors: u64,
    active_threads: u32,
    elapsed_ms: u64,
    current_rps: f64,
    mean_ms: f64,
    p95_ms: u64,
    min_ms: u64,
    max_ms: u64,
}

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn parse_uuid(s: &str) -> Result<Uuid, RmeterError> {
    Uuid::parse_str(s)
        .map_err(|e| RmeterError::Validation(format!("Invalid UUID '{}': {}", s, e)))
}

fn lock_engine<'a>(
    state: &'a State<'_, Mutex<EngineState>>,
) -> Result<std::sync::MutexGuard<'a, EngineState>, RmeterError> {
    state
        .lock()
        .map_err(|e| RmeterError::Internal(format!("EngineState mutex poisoned: {e}")))
}

fn lock_plans<'a>(
    manager: &'a State<'_, Mutex<PlanManager>>,
) -> Result<std::sync::MutexGuard<'a, PlanManager>, RmeterError> {
    manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))
}

fn lock_result_store<'a>(
    store: &'a State<'_, Mutex<ResultStore>>,
) -> Result<std::sync::MutexGuard<'a, ResultStore>, RmeterError> {
    store
        .lock()
        .map_err(|e| RmeterError::Internal(format!("ResultStore mutex poisoned: {e}")))
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Start a test run for the given plan ID.
///
/// The engine runs in the background and emits Tauri events:
/// - `"test-result"` — [`RequestResultEvent`] for every completed request
/// - `"test-progress"` — periodic throughput/latency snapshot (~500 ms)
/// - `"test-status"` — [`EngineStatus`] change notifications
/// - `"test-complete"` — final [`TestSummary`] when the run finishes
///
/// On completion, the full [`TestRunResult`] is persisted in the
/// [`ResultStore`] managed state for later export and comparison.
#[tauri::command]
pub async fn start_test(
    plan_id: String,
    app: AppHandle,
    engine_state: State<'_, Mutex<EngineState>>,
    plan_manager: State<'_, Mutex<PlanManager>>,
    result_store: State<'_, Mutex<ResultStore>>,
) -> Result<(), RmeterError> {
    let plan_uuid = parse_uuid(&plan_id)?;

    // Retrieve the plan (hold the mutex as briefly as possible).
    let plan = {
        let mgr = lock_plans(&plan_manager)?;
        mgr.get_plan(&plan_uuid)
            .cloned()
            .ok_or_else(|| RmeterError::PlanNotFound(plan_id.clone()))?
    };

    // Validate the plan before running.
    let validation_errors = validate_plan(&plan);
    if !validation_errors.is_empty() {
        let messages: Vec<String> = validation_errors.iter().map(|e| e.to_string()).collect();
        return Err(RmeterError::Validation(format!(
            "Plan validation failed: {}",
            messages.join("; ")
        )));
    }

    // Reject if an engine is already running.
    {
        let state = lock_engine(&engine_state)?;
        if let Some(ref handle) = state.handle {
            let current = handle.status.try_read();
            if let Ok(s) = current {
                if *s == EngineStatus::Running || *s == EngineStatus::Stopping {
                    return Err(RmeterError::Engine(
                        "A test is already running. Stop it before starting a new one."
                            .to_string(),
                    ));
                }
            }
        }
    }

    // Create the event channel (bounded to avoid runaway memory use).
    let (tx, mut rx) = mpsc::channel::<EngineEvent>(4096);

    let config = EngineConfig {
        plan,
        result_tx: tx,
    };

    // Start the engine.
    let engine_handle = run_test(config).await?;

    // Capture the aggregator reference for later `get_current_stats` calls.
    let aggregator_ref = Arc::clone(&engine_handle.aggregator);

    // Store the handle.
    {
        let mut state = lock_engine(&engine_state)?;
        state.handle = Some(engine_handle);
        state.aggregator = Some(Arc::clone(&aggregator_ref));
    }

    // Verify the ResultStore lock is healthy before spawning the bridge task.
    {
        let _guard = lock_result_store(&result_store)?;
    }

    // Spawn a task that bridges engine events to Tauri events on the app window
    // and collects request results for post-run storage.
    //
    // We clone the AppHandle (which is 'static + Clone) and call app.state()
    // inside the task to access the managed ResultStore without borrowing State.
    let app_clone = app.clone();
    let agg_for_bridge = Arc::clone(&aggregator_ref);
    tokio::spawn(async move {
        let mut collected_results: Vec<RequestResultEvent> = Vec::new();

        while let Some(event) = rx.recv().await {
            match event {
                EngineEvent::RequestResult(ref result) => {
                    collected_results.push(result.clone());
                    let _ = app_clone.emit("test-result", result);
                }
                EngineEvent::Progress {
                    completed_requests,
                    total_errors,
                    active_threads,
                    elapsed_ms,
                    current_rps,
                    mean_ms,
                    p95_ms,
                    min_ms,
                    max_ms,
                } => {
                    let _ = app_clone.emit(
                        "test-progress",
                        ProgressPayload {
                            completed_requests,
                            total_errors,
                            active_threads,
                            elapsed_ms,
                            current_rps,
                            mean_ms,
                            p95_ms,
                            min_ms,
                            max_ms,
                        },
                    );
                }
                EngineEvent::StatusChange { ref status } => {
                    let _ = app_clone.emit("test-status", status);
                }
                EngineEvent::Complete { ref summary } => {
                    // Build the full test run result for storage.
                    let time_series = agg_for_bridge.read().await.time_series();
                    let run_result = TestRunResult {
                        run_id: Uuid::new_v4(),
                        summary: summary.clone(),
                        time_series,
                        request_results: std::mem::take(&mut collected_results),
                    };

                    // Persist the result via the app's managed state.
                    // app.state::<T>() borrows are valid for the app's lifetime
                    // and AppHandle itself is 'static, so this is safe to call
                    // from inside the spawned task.
                    {
                        let store_state: tauri::State<'_, Mutex<ResultStore>> =
                            app_clone.state();
                        if let Ok(mut store) = store_state.lock() {
                            store.add(run_result);
                        };
                    }

                    let _ = app_clone.emit("test-complete", summary);
                }
            }
        }
    });

    Ok(())
}

/// Request a graceful stop: the engine will finish in-flight requests then
/// produce a final summary.
#[tauri::command]
pub async fn stop_test(
    engine_state: State<'_, Mutex<EngineState>>,
) -> Result<(), RmeterError> {
    // Extract what we need from the MutexGuard, then drop it before awaiting.
    let (cancel_token, status_arc) = {
        let state = lock_engine(&engine_state)?;
        match state.handle {
            Some(ref handle) => (handle.cancel_token.clone(), Arc::clone(&handle.status)),
            None => {
                return Err(RmeterError::Engine(
                    "No test is currently running".to_string(),
                ))
            }
        }
    };

    cancel_token.cancel();
    let mut s = status_arc.write().await;
    *s = EngineStatus::Stopping;
    Ok(())
}

/// Immediately cancel the engine without waiting for in-flight requests.
/// Equivalent to `stop_test` in this implementation (CancellationToken is
/// checked between requests, not mid-request).
#[tauri::command]
pub async fn force_stop_test(
    engine_state: State<'_, Mutex<EngineState>>,
) -> Result<(), RmeterError> {
    stop_test(engine_state).await
}

/// Return the current lifecycle status of the engine.
#[tauri::command]
pub async fn get_engine_status(
    engine_state: State<'_, Mutex<EngineState>>,
) -> Result<EngineStatus, RmeterError> {
    let status_arc = {
        let state = lock_engine(&engine_state)?;
        match state.handle {
            Some(ref handle) => Some(Arc::clone(&handle.status)),
            None => None,
        }
    };

    if let Some(arc) = status_arc {
        let s = arc.read().await;
        Ok(s.clone())
    } else {
        Ok(EngineStatus::Idle)
    }
}

/// Return a live statistics snapshot (usable during and after execution).
///
/// Returns `None` when no test has been started yet.
#[tauri::command]
pub async fn get_current_stats(
    engine_state: State<'_, Mutex<EngineState>>,
) -> Result<Option<AggregatorSnapshot>, RmeterError> {
    let agg_arc = {
        let state = lock_engine(&engine_state)?;
        state.aggregator.as_ref().map(Arc::clone)
    };

    if let Some(agg) = agg_arc {
        let snapshot = agg.read().await.snapshot();
        Ok(Some(snapshot))
    } else {
        Ok(None)
    }
}

/// Return the per-second time-series data for the current (or last) test run.
///
/// Returns an empty vec when no test has been started yet.
#[tauri::command]
pub async fn get_time_series(
    engine_state: State<'_, Mutex<EngineState>>,
) -> Result<Vec<TimeBucketEntry>, RmeterError> {
    let agg_arc = {
        let state = lock_engine(&engine_state)?;
        state.aggregator.as_ref().map(Arc::clone)
    };

    if let Some(agg) = agg_arc {
        let series = agg.read().await.time_series();
        Ok(series)
    } else {
        Ok(Vec::new())
    }
}
