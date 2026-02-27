pub mod export;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Aggregated summary of a completed test plan execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TestSummary {
    pub plan_id: Uuid,
    pub plan_name: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    /// Total number of HTTP requests that were sent.
    pub total_requests: u64,
    /// Number of requests that completed without network-level errors.
    pub successful_requests: u64,
    /// Number of requests that resulted in network-level errors.
    pub failed_requests: u64,
    /// Minimum response time observed across all requests (ms).
    pub min_response_ms: u64,
    /// Maximum response time observed across all requests (ms).
    pub max_response_ms: u64,
    /// Mean response time across all requests (ms).
    pub mean_response_ms: f64,
    /// 50th percentile response time (ms).
    pub p50_response_ms: u64,
    /// 95th percentile response time (ms).
    pub p95_response_ms: u64,
    /// 99th percentile response time (ms).
    pub p99_response_ms: u64,
    /// Aggregate throughput in requests per second.
    pub requests_per_second: f64,
    /// Total bytes received across all responses.
    pub total_bytes_received: u64,
}

/// A single request result event emitted during test execution.
///
/// These events are streamed to the frontend via Tauri events.
/// Maximum length of response body stored per request result.
pub const MAX_RESPONSE_BODY_LEN: usize = 4096;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct RequestResultEvent {
    pub id: Uuid,
    pub plan_id: Uuid,
    pub thread_group_name: String,
    pub request_name: String,
    pub timestamp: DateTime<Utc>,
    pub status_code: u16,
    pub elapsed_ms: u64,
    pub size_bytes: u64,
    /// Whether all configured assertions passed (or none were configured).
    pub assertions_passed: bool,
    /// Human-readable error message if the request failed at the network level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Per-assertion evaluation results.  Empty when no assertions are configured.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub assertion_results: Vec<crate::assertions::AssertionResult>,
    /// Per-extractor evaluation results.  Empty when no extractors are configured.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extraction_results: Vec<crate::extractors::ExtractionResult>,
    // -- Request/response detail fields for inspection --
    /// HTTP method used (e.g. "GET", "POST").
    #[serde(default)]
    pub method: String,
    /// The resolved URL that was requested.
    #[serde(default)]
    pub url: String,
    /// Response headers (lowercased keys).
    #[serde(default, skip_serializing_if = "std::collections::HashMap::is_empty")]
    pub response_headers: std::collections::HashMap<String, String>,
    /// Response body (truncated to 4 KB).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_body: Option<String>,
}

// ---------------------------------------------------------------------------
// TestRunResult — complete data for a finished test run
// ---------------------------------------------------------------------------

/// Complete results of a finished test run, suitable for export.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TestRunResult {
    /// Unique ID for this test run.
    pub run_id: Uuid,
    /// Aggregated summary statistics.
    pub summary: TestSummary,
    /// Per-second time-series data.
    pub time_series: Vec<crate::engine::aggregator::TimeBucketEntry>,
    /// All individual request results collected during the run.
    pub request_results: Vec<RequestResultEvent>,
}

// ---------------------------------------------------------------------------
// ResultSummaryEntry — lightweight list entry
// ---------------------------------------------------------------------------

/// Lightweight entry for listing stored test run results.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ResultSummaryEntry {
    pub run_id: Uuid,
    pub plan_name: String,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub total_requests: u64,
    pub requests_per_second: f64,
    pub mean_response_ms: f64,
    pub error_rate: f64,
}

impl ResultSummaryEntry {
    pub fn from_run(run: &TestRunResult) -> Self {
        let s = &run.summary;
        let error_rate = if s.total_requests > 0 {
            s.failed_requests as f64 / s.total_requests as f64
        } else {
            0.0
        };
        Self {
            run_id: run.run_id,
            plan_name: s.plan_name.clone(),
            started_at: s.started_at,
            finished_at: s.finished_at,
            total_requests: s.total_requests,
            requests_per_second: s.requests_per_second,
            mean_response_ms: s.mean_response_ms,
            error_rate,
        }
    }
}

// ---------------------------------------------------------------------------
// ComparisonResult — delta between two test runs
// ---------------------------------------------------------------------------

/// Comparison of two test run summaries with computed deltas.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ComparisonResult {
    pub run_a: TestSummary,
    pub run_b: TestSummary,
    pub delta_total_requests: i64,
    pub delta_mean_ms: f64,
    pub delta_p95_ms: i64,
    pub delta_p99_ms: i64,
    pub delta_rps: f64,
    pub delta_error_rate: f64,
}

pub fn compare_results(a: &TestRunResult, b: &TestRunResult) -> ComparisonResult {
    let error_rate_a = if a.summary.total_requests > 0 {
        a.summary.failed_requests as f64 / a.summary.total_requests as f64
    } else {
        0.0
    };
    let error_rate_b = if b.summary.total_requests > 0 {
        b.summary.failed_requests as f64 / b.summary.total_requests as f64
    } else {
        0.0
    };

    ComparisonResult {
        run_a: a.summary.clone(),
        run_b: b.summary.clone(),
        delta_total_requests: b.summary.total_requests as i64
            - a.summary.total_requests as i64,
        delta_mean_ms: b.summary.mean_response_ms - a.summary.mean_response_ms,
        delta_p95_ms: b.summary.p95_response_ms as i64
            - a.summary.p95_response_ms as i64,
        delta_p99_ms: b.summary.p99_response_ms as i64
            - a.summary.p99_response_ms as i64,
        delta_rps: b.summary.requests_per_second - a.summary.requests_per_second,
        delta_error_rate: error_rate_b - error_rate_a,
    }
}

// ---------------------------------------------------------------------------
// ResultStore — in-memory store for completed runs
// ---------------------------------------------------------------------------

/// In-memory store for completed test run results.
/// Holds the last `max_runs` completed runs.
pub struct ResultStore {
    runs: Vec<TestRunResult>,
    max_runs: usize,
}

impl ResultStore {
    pub fn new(max_runs: usize) -> Self {
        Self {
            runs: Vec::new(),
            max_runs,
        }
    }

    /// Add a completed run, evicting the oldest if the store is at capacity.
    pub fn add(&mut self, result: TestRunResult) {
        if self.runs.len() >= self.max_runs {
            self.runs.remove(0);
        }
        self.runs.push(result);
    }

    /// Return lightweight summary entries for all stored runs.
    pub fn list(&self) -> Vec<ResultSummaryEntry> {
        self.runs.iter().map(ResultSummaryEntry::from_run).collect()
    }

    /// Look up a run by its UUID.
    pub fn get(&self, run_id: &Uuid) -> Option<&TestRunResult> {
        self.runs.iter().find(|r| r.run_id == *run_id)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::results::export::{export_csv, export_html, export_json};

    /// Build a minimal [`TestSummary`] for use in tests.
    fn make_summary(plan_name: &str, total: u64, failed: u64, mean_ms: f64) -> TestSummary {
        let now = Utc::now();
        TestSummary {
            plan_id: Uuid::new_v4(),
            plan_name: plan_name.to_string(),
            started_at: now,
            finished_at: now,
            total_requests: total,
            successful_requests: total.saturating_sub(failed),
            failed_requests: failed,
            min_response_ms: 10,
            max_response_ms: 500,
            mean_response_ms: mean_ms,
            p50_response_ms: 100,
            p95_response_ms: 300,
            p99_response_ms: 490,
            requests_per_second: if total > 0 { total as f64 } else { 0.0 },
            total_bytes_received: total * 1024,
        }
    }

    /// Build a minimal [`RequestResultEvent`] for use in tests.
    fn make_result_event(
        plan_id: Uuid,
        name: &str,
        group: &str,
        status: u16,
        elapsed_ms: u64,
        success: bool,
    ) -> RequestResultEvent {
        RequestResultEvent {
            id: Uuid::new_v4(),
            plan_id,
            thread_group_name: group.to_string(),
            request_name: name.to_string(),
            timestamp: Utc::now(),
            status_code: status,
            elapsed_ms,
            size_bytes: 512,
            assertions_passed: success,
            error: if success { None } else { Some("connection refused".to_string()) },
            assertion_results: Vec::new(),
            extraction_results: Vec::new(),
            method: "GET".to_string(),
            url: format!("http://example.com/{name}"),
            response_headers: std::collections::HashMap::new(),
            response_body: None,
        }
    }

    /// Build a [`TestRunResult`] with the given summary and request results.
    fn make_run(summary: TestSummary, events: Vec<RequestResultEvent>) -> TestRunResult {
        TestRunResult {
            run_id: Uuid::new_v4(),
            summary,
            time_series: Vec::new(),
            request_results: events,
        }
    }

    // -----------------------------------------------------------------------
    // ResultStore
    // -----------------------------------------------------------------------

    #[test]
    fn result_store_add_and_get() {
        let mut store = ResultStore::new(10);
        let summary = make_summary("Plan A", 100, 0, 50.0);
        let run = make_run(summary, Vec::new());
        let run_id = run.run_id;
        store.add(run);
        let retrieved = store.get(&run_id).expect("run should be present");
        assert_eq!(retrieved.run_id, run_id);
    }

    #[test]
    fn result_store_get_returns_none_for_missing() {
        let store = ResultStore::new(10);
        let missing = Uuid::new_v4();
        assert!(store.get(&missing).is_none());
    }

    #[test]
    fn result_store_list_returns_summaries_in_insertion_order() {
        let mut store = ResultStore::new(10);
        for i in 0..3u64 {
            let summary = make_summary(&format!("Plan {i}"), 10 * (i + 1), 0, 50.0);
            store.add(make_run(summary, Vec::new()));
        }
        let list = store.list();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].plan_name, "Plan 0");
        assert_eq!(list[1].plan_name, "Plan 1");
        assert_eq!(list[2].plan_name, "Plan 2");
    }

    #[test]
    fn result_store_respects_max_runs_limit() {
        let mut store = ResultStore::new(3);
        for i in 0..5u64 {
            let summary = make_summary(&format!("Run {i}"), 100, 0, 50.0);
            store.add(make_run(summary, Vec::new()));
        }
        // Should only keep the last 3 (oldest 2 evicted).
        let list = store.list();
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].plan_name, "Run 2");
        assert_eq!(list[1].plan_name, "Run 3");
        assert_eq!(list[2].plan_name, "Run 4");
    }

    #[test]
    fn result_store_error_rate_calculation() {
        let mut store = ResultStore::new(10);
        let summary = make_summary("Plan", 100, 10, 50.0);
        let run = make_run(summary, Vec::new());
        store.add(run);
        let list = store.list();
        assert!((list[0].error_rate - 0.1).abs() < 0.001);
    }

    #[test]
    fn result_store_zero_requests_error_rate_is_zero() {
        let mut store = ResultStore::new(10);
        let summary = make_summary("Empty Plan", 0, 0, 0.0);
        store.add(make_run(summary, Vec::new()));
        let list = store.list();
        assert_eq!(list[0].error_rate, 0.0);
    }

    // -----------------------------------------------------------------------
    // compare_results
    // -----------------------------------------------------------------------

    #[test]
    fn compare_results_positive_deltas() {
        let summary_a = make_summary("Plan A", 100, 5, 100.0);
        let summary_b = make_summary("Plan B", 150, 10, 120.0);
        let run_a = make_run(summary_a, Vec::new());
        let run_b = make_run(summary_b, Vec::new());

        let cmp = compare_results(&run_a, &run_b);
        assert_eq!(cmp.delta_total_requests, 50);
        assert!((cmp.delta_mean_ms - 20.0).abs() < 0.001);
    }

    #[test]
    fn compare_results_negative_deltas() {
        let summary_a = make_summary("Plan A", 200, 0, 150.0);
        let summary_b = make_summary("Plan B", 100, 0, 80.0);
        let run_a = make_run(summary_a, Vec::new());
        let run_b = make_run(summary_b, Vec::new());

        let cmp = compare_results(&run_a, &run_b);
        assert_eq!(cmp.delta_total_requests, -100);
        assert!((cmp.delta_mean_ms - (-70.0)).abs() < 0.001);
    }

    #[test]
    fn compare_results_error_rate_delta() {
        // Run A: 0 errors, Run B: 10% error rate.
        let mut summary_a = make_summary("A", 100, 0, 50.0);
        summary_a.failed_requests = 0;
        let mut summary_b = make_summary("B", 100, 10, 50.0);
        summary_b.failed_requests = 10;
        let run_a = make_run(summary_a, Vec::new());
        let run_b = make_run(summary_b, Vec::new());

        let cmp = compare_results(&run_a, &run_b);
        assert!((cmp.delta_error_rate - 0.1).abs() < 0.001);
    }

    #[test]
    fn compare_results_p95_p99_deltas() {
        let mut summary_a = make_summary("A", 100, 0, 50.0);
        summary_a.p95_response_ms = 200;
        summary_a.p99_response_ms = 400;
        let mut summary_b = make_summary("B", 100, 0, 50.0);
        summary_b.p95_response_ms = 350;
        summary_b.p99_response_ms = 490;
        let run_a = make_run(summary_a, Vec::new());
        let run_b = make_run(summary_b, Vec::new());

        let cmp = compare_results(&run_a, &run_b);
        assert_eq!(cmp.delta_p95_ms, 150);
        assert_eq!(cmp.delta_p99_ms, 90);
    }

    // -----------------------------------------------------------------------
    // ResultSummaryEntry
    // -----------------------------------------------------------------------

    #[test]
    fn result_summary_entry_from_run_fields() {
        let summary = make_summary("Test Plan", 200, 4, 75.5);
        let run = make_run(summary, Vec::new());
        let entry = ResultSummaryEntry::from_run(&run);
        assert_eq!(entry.plan_name, "Test Plan");
        assert_eq!(entry.total_requests, 200);
        assert!((entry.error_rate - 0.02).abs() < 0.001);
        assert!((entry.mean_response_ms - 75.5).abs() < 0.001);
    }

    // -----------------------------------------------------------------------
    // Export: CSV
    // -----------------------------------------------------------------------

    #[test]
    fn export_csv_contains_header_row() {
        let summary = make_summary("My Plan", 2, 0, 50.0);
        let plan_id = summary.plan_id;
        let events = vec![
            make_result_event(plan_id, "Login", "Group A", 200, 100, true),
            make_result_event(plan_id, "Logout", "Group A", 200, 80, true),
        ];
        let run = make_run(summary, events);
        let csv = export_csv(&run);
        assert!(csv.contains("timestamp,request_name,thread_group,status_code,elapsed_ms,size_bytes,success,error"));
    }

    #[test]
    fn export_csv_contains_plan_name_in_comment() {
        let summary = make_summary("My Plan", 0, 0, 0.0);
        let run = make_run(summary, Vec::new());
        let csv = export_csv(&run);
        assert!(csv.contains("My Plan"));
    }

    #[test]
    fn export_csv_one_data_row_per_event() {
        let summary = make_summary("Plan", 3, 0, 50.0);
        let plan_id = summary.plan_id;
        let events = vec![
            make_result_event(plan_id, "R1", "G", 200, 50, true),
            make_result_event(plan_id, "R2", "G", 201, 60, true),
            make_result_event(plan_id, "R3", "G", 500, 70, false),
        ];
        let run = make_run(summary, events);
        let csv = export_csv(&run);
        // Count data lines (non-comment, non-empty, non-header lines).
        let data_lines: Vec<&str> = csv
            .lines()
            .filter(|l| !l.starts_with('#') && !l.is_empty() && !l.starts_with("timestamp"))
            .collect();
        assert_eq!(data_lines.len(), 3);
    }

    #[test]
    fn export_csv_failed_event_shows_false() {
        let summary = make_summary("Plan", 1, 1, 50.0);
        let plan_id = summary.plan_id;
        let events = vec![make_result_event(plan_id, "Fail", "G", 500, 100, false)];
        let run = make_run(summary, events);
        let csv = export_csv(&run);
        assert!(csv.contains("false"));
    }

    // -----------------------------------------------------------------------
    // Export: JSON
    // -----------------------------------------------------------------------

    #[test]
    fn export_json_is_valid_json() {
        let summary = make_summary("Plan", 10, 0, 50.0);
        let plan_id = summary.plan_id;
        let events = vec![make_result_event(plan_id, "R", "G", 200, 100, true)];
        let run = make_run(summary, events);
        let json_str = export_json(&run).expect("export_json should not fail");
        let parsed: serde_json::Value =
            serde_json::from_str(&json_str).expect("output should be valid JSON");
        assert!(parsed.get("run_id").is_some());
        assert!(parsed.get("summary").is_some());
        assert!(parsed.get("request_results").is_some());
    }

    #[test]
    fn export_json_contains_plan_name() {
        let summary = make_summary("My Special Plan", 5, 0, 20.0);
        let run = make_run(summary, Vec::new());
        let json_str = export_json(&run).expect("export_json should not fail");
        assert!(json_str.contains("My Special Plan"));
    }

    // -----------------------------------------------------------------------
    // Export: HTML
    // -----------------------------------------------------------------------

    #[test]
    fn export_html_is_valid_html_document() {
        let summary = make_summary("HTML Plan", 10, 1, 55.0);
        let plan_id = summary.plan_id;
        let events = vec![
            make_result_event(plan_id, "Get Users", "Workers", 200, 45, true),
            make_result_event(plan_id, "Create User", "Workers", 500, 200, false),
        ];
        let run = make_run(summary, events);
        let html = export_html(&run);
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<html"));
        assert!(html.contains("</html>"));
    }

    #[test]
    fn export_html_contains_plan_name() {
        let summary = make_summary("My HTML Plan", 5, 0, 30.0);
        let run = make_run(summary, Vec::new());
        let html = export_html(&run);
        assert!(html.contains("My HTML Plan"));
    }

    #[test]
    fn export_html_contains_summary_section() {
        let summary = make_summary("Plan", 100, 5, 80.0);
        let run = make_run(summary, Vec::new());
        let html = export_html(&run);
        assert!(html.contains("Summary"));
        assert!(html.contains("Total Requests"));
    }

    #[test]
    fn export_html_contains_individual_requests_section() {
        let summary = make_summary("Plan", 1, 0, 50.0);
        let plan_id = summary.plan_id;
        let events = vec![make_result_event(plan_id, "Login", "G", 200, 50, true)];
        let run = make_run(summary, events);
        let html = export_html(&run);
        assert!(html.contains("Individual Requests"));
        assert!(html.contains("Login"));
    }

    #[test]
    fn export_html_escapes_special_chars_in_plan_name() {
        let summary = make_summary("Plan <A> & B", 0, 0, 0.0);
        let run = make_run(summary, Vec::new());
        let html = export_html(&run);
        // The plan name should be HTML-escaped in the output.
        assert!(html.contains("&lt;A&gt;") || html.contains("&amp;"));
    }
}
