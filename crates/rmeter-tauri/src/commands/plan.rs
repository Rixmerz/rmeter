use std::sync::Mutex;

use rmeter_core::error::RmeterError;
use rmeter_core::plan::io::{read_plan, write_plan};
use rmeter_core::plan::manager::{HttpRequestUpdate, PlanManager, PlanSummary, ThreadGroupUpdate};
use rmeter_core::plan::model::{CsvSharingMode, HttpRequest, TestPlan, ThreadGroup, VariableScope};
use rmeter_core::plan::templates;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Helper: parse a String into a Uuid
// ---------------------------------------------------------------------------

fn parse_uuid(s: &str) -> Result<Uuid, RmeterError> {
    Uuid::parse_str(s).map_err(|e| {
        RmeterError::Validation(format!("Invalid UUID '{}': {}", s, e))
    })
}

// ---------------------------------------------------------------------------
// Plan CRUD
// ---------------------------------------------------------------------------

/// Create a new empty test plan and return the full plan.
#[tauri::command]
pub fn create_plan(
    name: String,
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<TestPlan, RmeterError> {
    let mut mgr = manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;

    let id = mgr.create_plan(name);
    let plan = mgr
        .get_plan(&id)
        .ok_or_else(|| RmeterError::Internal("Plan disappeared immediately after creation".to_string()))?
        .clone();
    Ok(plan)
}

/// Return the full test plan for the given ID.
#[tauri::command]
pub fn get_plan(
    id: String,
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<TestPlan, RmeterError> {
    let id = parse_uuid(&id)?;
    let mgr = manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;

    mgr.get_plan(&id)
        .cloned()
        .ok_or_else(|| RmeterError::PlanNotFound(id.to_string()))
}

/// Return lightweight summaries of all loaded plans.
#[tauri::command]
pub fn list_plans(
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<Vec<PlanSummary>, RmeterError> {
    let mgr = manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;

    Ok(mgr.list_plans())
}

/// Delete a plan by ID.
#[tauri::command]
pub fn delete_plan(
    id: String,
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<(), RmeterError> {
    let id = parse_uuid(&id)?;
    let mut mgr = manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;

    if !mgr.delete_plan(&id) {
        return Err(RmeterError::PlanNotFound(id.to_string()));
    }
    Ok(())
}

/// Mark a plan as the active (currently selected) plan.
#[tauri::command]
pub fn set_active_plan(
    id: String,
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<(), RmeterError> {
    let id = parse_uuid(&id)?;
    let mut mgr = manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;

    // Ensure the plan actually exists before marking it active.
    if mgr.get_plan(&id).is_none() {
        return Err(RmeterError::PlanNotFound(id.to_string()));
    }
    mgr.set_active_plan(id);
    Ok(())
}

/// Return the currently active plan (if any).
#[tauri::command]
pub fn get_active_plan(
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<Option<TestPlan>, RmeterError> {
    let mgr = manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;

    Ok(mgr.get_active_plan().cloned())
}

// ---------------------------------------------------------------------------
// File I/O
// ---------------------------------------------------------------------------

/// Serialize a plan to disk at the given path.
#[tauri::command]
pub async fn save_plan(
    id: String,
    path: String,
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<(), RmeterError> {
    let id = parse_uuid(&id)?;

    let plan = {
        let mgr = manager
            .lock()
            .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;
        mgr.get_plan(&id)
            .cloned()
            .ok_or_else(|| RmeterError::PlanNotFound(id.to_string()))?
    };

    write_plan(&plan, &path).await?;
    Ok(())
}

/// Load a plan from disk, add it to the manager, and return the full plan.
#[tauri::command]
pub async fn load_plan(
    path: String,
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<TestPlan, RmeterError> {
    let plan = read_plan(&path).await?;

    let mut mgr = manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;

    let loaded = plan.clone();
    mgr.add_plan(plan);
    Ok(loaded)
}

// ---------------------------------------------------------------------------
// Thread Group operations
// ---------------------------------------------------------------------------

/// Add a new default thread group to the specified plan.
///
/// Returns the updated thread group.
#[tauri::command]
pub fn add_thread_group(
    plan_id: String,
    name: String,
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<ThreadGroup, RmeterError> {
    let plan_id = parse_uuid(&plan_id)?;
    let mut mgr = manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;

    let group_id = mgr.add_thread_group(&plan_id, name)?;
    let tg = mgr
        .get_plan(&plan_id)
        .and_then(|p| p.thread_groups.iter().find(|tg| tg.id == group_id))
        .cloned()
        .ok_or_else(|| {
            RmeterError::Internal("Thread group disappeared immediately after creation".to_string())
        })?;
    Ok(tg)
}

/// Remove a thread group from the specified plan.
#[tauri::command]
pub fn remove_thread_group(
    plan_id: String,
    group_id: String,
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<(), RmeterError> {
    let plan_id = parse_uuid(&plan_id)?;
    let group_id = parse_uuid(&group_id)?;
    let mut mgr = manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;

    mgr.remove_thread_group(&plan_id, &group_id)
}

/// Apply a partial update to a thread group and return the updated group.
#[tauri::command]
pub fn update_thread_group(
    plan_id: String,
    group_id: String,
    update: ThreadGroupUpdate,
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<ThreadGroup, RmeterError> {
    let plan_id = parse_uuid(&plan_id)?;
    let group_id = parse_uuid(&group_id)?;
    let mut mgr = manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;

    let tg = mgr.update_thread_group(&plan_id, &group_id, update)?;
    Ok(tg.clone())
}

// ---------------------------------------------------------------------------
// Request operations
// ---------------------------------------------------------------------------

/// Add a new default GET request to the specified thread group and return it.
#[tauri::command]
pub fn add_request(
    plan_id: String,
    group_id: String,
    name: String,
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<HttpRequest, RmeterError> {
    let plan_id = parse_uuid(&plan_id)?;
    let group_id = parse_uuid(&group_id)?;
    let mut mgr = manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;

    let request_id = mgr.add_request(&plan_id, &group_id, name)?;
    let req = mgr
        .get_plan(&plan_id)
        .and_then(|p| p.thread_groups.iter().find(|tg| tg.id == group_id))
        .and_then(|tg| tg.requests.iter().find(|r| r.id == request_id))
        .cloned()
        .ok_or_else(|| {
            RmeterError::Internal("Request disappeared immediately after creation".to_string())
        })?;
    Ok(req)
}

/// Remove a request from the specified thread group.
#[tauri::command]
pub fn remove_request(
    plan_id: String,
    group_id: String,
    request_id: String,
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<(), RmeterError> {
    let plan_id = parse_uuid(&plan_id)?;
    let group_id = parse_uuid(&group_id)?;
    let request_id = parse_uuid(&request_id)?;
    let mut mgr = manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;

    mgr.remove_request(&plan_id, &group_id, &request_id)
}

/// Apply a partial update to a request and return the updated request.
#[tauri::command]
pub fn update_request(
    plan_id: String,
    group_id: String,
    request_id: String,
    update: HttpRequestUpdate,
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<HttpRequest, RmeterError> {
    let plan_id = parse_uuid(&plan_id)?;
    let group_id = parse_uuid(&group_id)?;
    let request_id = parse_uuid(&request_id)?;
    let mut mgr = manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;

    let req = mgr.update_request(&plan_id, &group_id, &request_id, update)?;
    Ok(req.clone())
}

// ---------------------------------------------------------------------------
// Utility commands
// ---------------------------------------------------------------------------

/// Duplicate a thread group or request (identified by element_id) within the
/// given plan.
///
/// Searches thread groups first; if not found, searches requests in all groups.
/// Returns the full updated plan.
#[tauri::command]
pub fn duplicate_element(
    plan_id: String,
    element_id: String,
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<TestPlan, RmeterError> {
    let plan_id = parse_uuid(&plan_id)?;
    let element_id = parse_uuid(&element_id)?;
    let mut mgr = manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;

    // Determine whether element_id is a thread group or a request.
    let is_thread_group = mgr
        .get_plan(&plan_id)
        .ok_or_else(|| RmeterError::PlanNotFound(plan_id.to_string()))?
        .thread_groups
        .iter()
        .any(|tg| tg.id == element_id);

    if is_thread_group {
        mgr.duplicate_thread_group(&plan_id, &element_id)?;
    } else {
        // Find which thread group owns this request.
        let group_id = mgr
            .get_plan(&plan_id)
            .and_then(|p| {
                p.thread_groups
                    .iter()
                    .find(|tg| tg.requests.iter().any(|r| r.id == element_id))
                    .map(|tg| tg.id)
            })
            .ok_or_else(|| {
                RmeterError::PlanNotFound(format!(
                    "Element {} not found in plan {}",
                    element_id, plan_id
                ))
            })?;
        mgr.duplicate_request(&plan_id, &group_id, &element_id)?;
    }

    let plan = mgr
        .get_plan(&plan_id)
        .cloned()
        .ok_or_else(|| RmeterError::PlanNotFound(plan_id.to_string()))?;
    Ok(plan)
}

/// Reorder thread groups within a plan.
#[tauri::command]
pub fn reorder_thread_groups(
    plan_id: String,
    group_ids: Vec<String>,
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<(), RmeterError> {
    let plan_id = parse_uuid(&plan_id)?;
    let group_ids: Vec<Uuid> = group_ids
        .iter()
        .map(|s| parse_uuid(s))
        .collect::<Result<Vec<_>, _>>()?;

    let mut mgr = manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;

    mgr.reorder_thread_groups(&plan_id, group_ids)
}

/// Reorder requests within a thread group.
#[tauri::command]
pub fn reorder_requests(
    plan_id: String,
    group_id: String,
    request_ids: Vec<String>,
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<(), RmeterError> {
    let plan_id = parse_uuid(&plan_id)?;
    let group_id = parse_uuid(&group_id)?;
    let request_ids: Vec<Uuid> = request_ids
        .iter()
        .map(|s| parse_uuid(s))
        .collect::<Result<Vec<_>, _>>()?;

    let mut mgr = manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;

    mgr.reorder_requests(&plan_id, &group_id, request_ids)
}

/// Toggle the `enabled` flag of a thread group or request.
///
/// Returns the new enabled state.
#[tauri::command]
pub fn toggle_element(
    plan_id: String,
    element_id: String,
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<bool, RmeterError> {
    let plan_id = parse_uuid(&plan_id)?;
    let element_id = parse_uuid(&element_id)?;
    let mut mgr = manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;

    mgr.toggle_enabled(&plan_id, &element_id)
}

/// Rename a thread group or request.
#[tauri::command]
pub fn rename_element(
    plan_id: String,
    element_id: String,
    new_name: String,
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<(), RmeterError> {
    let plan_id = parse_uuid(&plan_id)?;
    let element_id = parse_uuid(&element_id)?;
    let mut mgr = manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;

    mgr.rename_element(&plan_id, &element_id, new_name)
}

// ---------------------------------------------------------------------------
// Assertion operations
// ---------------------------------------------------------------------------

/// Add a new assertion to the specified request and return the created assertion as JSON.
#[tauri::command]
pub fn add_assertion(
    plan_id: String,
    group_id: String,
    request_id: String,
    name: String,
    rule: serde_json::Value,
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<serde_json::Value, RmeterError> {
    let plan_id = parse_uuid(&plan_id)?;
    let group_id = parse_uuid(&group_id)?;
    let request_id = parse_uuid(&request_id)?;
    let mut mgr = manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;

    let assertion = mgr.add_assertion(&plan_id, &group_id, &request_id, name, rule)?;
    Ok(serde_json::to_value(&assertion)?)
}

/// Remove an assertion from the specified request.
#[tauri::command]
pub fn remove_assertion(
    plan_id: String,
    group_id: String,
    request_id: String,
    assertion_id: String,
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<(), RmeterError> {
    let plan_id = parse_uuid(&plan_id)?;
    let group_id = parse_uuid(&group_id)?;
    let request_id = parse_uuid(&request_id)?;
    let assertion_id = parse_uuid(&assertion_id)?;
    let mut mgr = manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;

    mgr.remove_assertion(&plan_id, &group_id, &request_id, &assertion_id)
}

/// Apply a partial update to an assertion and return the updated assertion as JSON.
#[tauri::command]
pub fn update_assertion(
    plan_id: String,
    group_id: String,
    request_id: String,
    assertion_id: String,
    name: Option<String>,
    rule: Option<serde_json::Value>,
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<serde_json::Value, RmeterError> {
    let plan_id = parse_uuid(&plan_id)?;
    let group_id = parse_uuid(&group_id)?;
    let request_id = parse_uuid(&request_id)?;
    let assertion_id = parse_uuid(&assertion_id)?;
    let mut mgr = manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;

    let assertion =
        mgr.update_assertion(&plan_id, &group_id, &request_id, &assertion_id, name, rule)?;
    Ok(serde_json::to_value(&assertion)?)
}

// ---------------------------------------------------------------------------
// Variable operations
// ---------------------------------------------------------------------------

/// Add a new variable to the specified plan and return the created variable as JSON.
#[tauri::command]
pub fn add_variable(
    plan_id: String,
    name: String,
    value: String,
    scope: VariableScope,
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<serde_json::Value, RmeterError> {
    let plan_id = parse_uuid(&plan_id)?;
    let mut mgr = manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;

    let variable = mgr.add_variable(&plan_id, name, value, scope)?;
    Ok(serde_json::to_value(&variable)?)
}

/// Remove a variable from the specified plan.
#[tauri::command]
pub fn remove_variable(
    plan_id: String,
    variable_id: String,
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<(), RmeterError> {
    let plan_id = parse_uuid(&plan_id)?;
    let variable_id = parse_uuid(&variable_id)?;
    let mut mgr = manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;

    mgr.remove_variable(&plan_id, &variable_id)
}

/// Apply a partial update to a variable and return the updated variable as JSON.
#[tauri::command]
pub fn update_variable(
    plan_id: String,
    variable_id: String,
    name: Option<String>,
    value: Option<String>,
    scope: Option<VariableScope>,
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<serde_json::Value, RmeterError> {
    let plan_id = parse_uuid(&plan_id)?;
    let variable_id = parse_uuid(&variable_id)?;
    let mut mgr = manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;

    let variable = mgr.update_variable(&plan_id, &variable_id, name, value, scope)?;
    Ok(serde_json::to_value(&variable)?)
}

// ---------------------------------------------------------------------------
// Extractor operations
// ---------------------------------------------------------------------------

/// Add a new extractor to the specified request and return the created extractor as JSON.
#[tauri::command]
pub fn add_extractor(
    plan_id: String,
    group_id: String,
    request_id: String,
    name: String,
    variable: String,
    expression: serde_json::Value,
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<serde_json::Value, RmeterError> {
    let plan_id = parse_uuid(&plan_id)?;
    let group_id = parse_uuid(&group_id)?;
    let request_id = parse_uuid(&request_id)?;
    let mut mgr = manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;

    let extractor =
        mgr.add_extractor(&plan_id, &group_id, &request_id, name, variable, expression)?;
    Ok(serde_json::to_value(&extractor)?)
}

/// Remove an extractor from the specified request.
#[tauri::command]
pub fn remove_extractor(
    plan_id: String,
    group_id: String,
    request_id: String,
    extractor_id: String,
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<(), RmeterError> {
    let plan_id = parse_uuid(&plan_id)?;
    let group_id = parse_uuid(&group_id)?;
    let request_id = parse_uuid(&request_id)?;
    let extractor_id = parse_uuid(&extractor_id)?;
    let mut mgr = manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;

    mgr.remove_extractor(&plan_id, &group_id, &request_id, &extractor_id)
}

/// Apply a partial update to an extractor and return the updated extractor as JSON.
#[tauri::command]
pub fn update_extractor(
    plan_id: String,
    group_id: String,
    request_id: String,
    extractor_id: String,
    name: Option<String>,
    variable: Option<String>,
    expression: Option<serde_json::Value>,
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<serde_json::Value, RmeterError> {
    let plan_id = parse_uuid(&plan_id)?;
    let group_id = parse_uuid(&group_id)?;
    let request_id = parse_uuid(&request_id)?;
    let extractor_id = parse_uuid(&extractor_id)?;
    let mut mgr = manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;

    let extractor = mgr.update_extractor(
        &plan_id,
        &group_id,
        &request_id,
        &extractor_id,
        name,
        variable,
        expression,
    )?;
    Ok(serde_json::to_value(&extractor)?)
}

// ---------------------------------------------------------------------------
// Templates
// ---------------------------------------------------------------------------

/// Create a new plan from a named template.
///
/// Valid values for `template`: `"rest_api"`, `"load_test"`, `"stress_test"`.
#[tauri::command]
pub fn create_from_template(
    template: String,
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<TestPlan, RmeterError> {
    let plan = match template.as_str() {
        "rest_api" => templates::rest_api_test(),
        "load_test" => templates::load_test(),
        "stress_test" => templates::stress_test(),
        other => {
            return Err(RmeterError::Validation(format!(
                "Unknown template '{}'. Valid options: rest_api, load_test, stress_test",
                other
            )))
        }
    };

    let mut mgr = manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;

    let created = plan.clone();
    mgr.add_plan(plan);
    Ok(created)
}

// ---------------------------------------------------------------------------
// CSV Data Source operations
// ---------------------------------------------------------------------------

/// Add a CSV data source to a plan by parsing raw CSV content.
///
/// The first row is treated as a header row with column names.
/// Each column name becomes a variable available as `${column_name}`.
#[tauri::command]
pub fn add_csv_data_source(
    plan_id: String,
    name: String,
    csv_content: String,
    delimiter: Option<String>,
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<serde_json::Value, RmeterError> {
    let plan_id = parse_uuid(&plan_id)?;
    let delim = delimiter
        .and_then(|d| d.as_bytes().first().copied())
        .unwrap_or(b',');

    let mut mgr = manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;

    let source = mgr.add_csv_data_source(&plan_id, name, csv_content, Some(delim))?;
    Ok(serde_json::to_value(&source)?)
}

/// Remove a CSV data source from a plan.
#[tauri::command]
pub fn remove_csv_data_source(
    plan_id: String,
    source_id: String,
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<(), RmeterError> {
    let plan_id = parse_uuid(&plan_id)?;
    let source_id = parse_uuid(&source_id)?;
    let mut mgr = manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;

    mgr.remove_csv_data_source(&plan_id, &source_id)
}

/// Update a CSV data source's metadata (name, sharing mode, recycle flag).
#[tauri::command]
pub fn update_csv_data_source(
    plan_id: String,
    source_id: String,
    name: Option<String>,
    sharing_mode: Option<CsvSharingMode>,
    recycle: Option<bool>,
    manager: tauri::State<'_, Mutex<PlanManager>>,
) -> Result<serde_json::Value, RmeterError> {
    let plan_id = parse_uuid(&plan_id)?;
    let source_id = parse_uuid(&source_id)?;
    let mut mgr = manager
        .lock()
        .map_err(|e| RmeterError::Internal(format!("PlanManager mutex poisoned: {e}")))?;

    let source = mgr.update_csv_data_source(&plan_id, &source_id, name, sharing_mode, recycle)?;
    Ok(serde_json::to_value(&source)?)
}
