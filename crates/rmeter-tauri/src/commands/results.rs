use std::sync::Mutex;

use rmeter_core::error::RmeterError;
use rmeter_core::results::{
    ComparisonResult, ResultStore, ResultSummaryEntry, TestRunResult,
    compare_results as core_compare_results,
    export::{export_csv, export_html, export_json},
};
use tauri::State;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn parse_uuid(s: &str) -> Result<Uuid, RmeterError> {
    Uuid::parse_str(s)
        .map_err(|e| RmeterError::Validation(format!("Invalid UUID '{}': {}", s, e)))
}

fn lock_store<'a>(
    store: &'a State<'_, Mutex<ResultStore>>,
) -> Result<std::sync::MutexGuard<'a, ResultStore>, RmeterError> {
    store
        .lock()
        .map_err(|e| RmeterError::Internal(format!("ResultStore mutex poisoned: {e}")))
}

fn get_run<'a>(
    guard: &'a std::sync::MutexGuard<'_, ResultStore>,
    run_id: &Uuid,
) -> Result<&'a TestRunResult, RmeterError> {
    guard
        .get(run_id)
        .ok_or_else(|| RmeterError::Internal(format!("Run not found: {}", run_id)))
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// List all stored test run results as lightweight summary entries.
#[tauri::command]
pub fn list_results(
    result_store: State<'_, Mutex<ResultStore>>,
) -> Result<Vec<ResultSummaryEntry>, RmeterError> {
    let store = lock_store(&result_store)?;
    Ok(store.list())
}

/// Retrieve a complete test run result by its run ID.
#[tauri::command]
pub fn get_result(
    run_id: String,
    result_store: State<'_, Mutex<ResultStore>>,
) -> Result<TestRunResult, RmeterError> {
    let id = parse_uuid(&run_id)?;
    let store = lock_store(&result_store)?;
    let run = get_run(&store, &id)?;
    Ok(run.clone())
}

/// Export a test run as a CSV string.
#[tauri::command]
pub fn export_results_csv(
    run_id: String,
    result_store: State<'_, Mutex<ResultStore>>,
) -> Result<String, RmeterError> {
    let id = parse_uuid(&run_id)?;
    let store = lock_store(&result_store)?;
    let run = get_run(&store, &id)?;
    Ok(export_csv(run))
}

/// Export a test run as a pretty-printed JSON string.
#[tauri::command]
pub fn export_results_json(
    run_id: String,
    result_store: State<'_, Mutex<ResultStore>>,
) -> Result<String, RmeterError> {
    let id = parse_uuid(&run_id)?;
    let store = lock_store(&result_store)?;
    let run = get_run(&store, &id)?;
    export_json(run).map_err(RmeterError::Serde)
}

/// Export a test run as a standalone HTML report string.
#[tauri::command]
pub fn export_results_html(
    run_id: String,
    result_store: State<'_, Mutex<ResultStore>>,
) -> Result<String, RmeterError> {
    let id = parse_uuid(&run_id)?;
    let store = lock_store(&result_store)?;
    let run = get_run(&store, &id)?;
    Ok(export_html(run))
}

/// Compare two test runs and return a delta report.
#[tauri::command]
pub fn compare_run_results(
    run_id_a: String,
    run_id_b: String,
    result_store: State<'_, Mutex<ResultStore>>,
) -> Result<ComparisonResult, RmeterError> {
    let id_a = parse_uuid(&run_id_a)?;
    let id_b = parse_uuid(&run_id_b)?;
    let store = lock_store(&result_store)?;
    let run_a = get_run(&store, &id_a)?.clone();
    let run_b = get_run(&store, &id_b)?.clone();
    Ok(core_compare_results(&run_a, &run_b))
}
