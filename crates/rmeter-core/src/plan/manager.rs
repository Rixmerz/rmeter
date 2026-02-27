use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::RmeterError;
use crate::plan::model::{
    Assertion, Extractor, HttpMethod, HttpRequest, LoopCount, RequestBody, TestPlan, ThreadGroup,
    Variable, VariableScope,
};

// ---------------------------------------------------------------------------
// Update DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
pub struct ThreadGroupUpdate {
    pub name: Option<String>,
    pub num_threads: Option<u32>,
    pub ramp_up_seconds: Option<u32>,
    pub loop_count: Option<LoopCount>,
    pub enabled: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HttpRequestUpdate {
    pub name: Option<String>,
    pub method: Option<HttpMethod>,
    pub url: Option<String>,
    pub headers: Option<HashMap<String, String>>,
    /// `Some(None)` clears the body; `Some(Some(body))` sets a new body.
    pub body: Option<Option<RequestBody>>,
    pub enabled: Option<bool>,
}

// ---------------------------------------------------------------------------
// PlanSummary
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct PlanSummary {
    pub id: Uuid,
    pub name: String,
    pub thread_group_count: usize,
    pub request_count: usize,
}

// ---------------------------------------------------------------------------
// PlanManager
// ---------------------------------------------------------------------------

/// In-memory manager for all currently open [`TestPlan`]s.
///
/// The manager tracks which plan is *active* (i.e. selected in the UI) and
/// provides CRUD operations for plans, thread groups, and requests without
/// touching the file system — persistence is handled separately via
/// [`crate::plan::io`].
#[derive(Debug, Default)]
pub struct PlanManager {
    plans: HashMap<Uuid, TestPlan>,
    active_plan_id: Option<Uuid>,
}

impl PlanManager {
    pub fn new() -> Self {
        Self::default()
    }

    // -----------------------------------------------------------------------
    // Plan CRUD
    // -----------------------------------------------------------------------

    /// Create a new empty [`TestPlan`] and return its generated ID.
    pub fn create_plan(&mut self, name: String) -> Uuid {
        let plan = TestPlan::new(name);
        let id = plan.id;
        self.plans.insert(id, plan);
        id
    }

    pub fn get_plan(&self, id: &Uuid) -> Option<&TestPlan> {
        self.plans.get(id)
    }

    pub fn get_plan_mut(&mut self, id: &Uuid) -> Option<&mut TestPlan> {
        self.plans.get_mut(id)
    }

    /// Remove a plan. Returns `true` if the plan existed and was removed.
    pub fn delete_plan(&mut self, id: &Uuid) -> bool {
        if self.active_plan_id == Some(*id) {
            self.active_plan_id = None;
        }
        self.plans.remove(id).is_some()
    }

    /// Return lightweight summaries for all loaded plans.
    pub fn list_plans(&self) -> Vec<PlanSummary> {
        let mut summaries: Vec<PlanSummary> = self
            .plans
            .values()
            .map(|p| PlanSummary {
                id: p.id,
                name: p.name.clone(),
                thread_group_count: p.thread_groups.len(),
                request_count: p.thread_groups.iter().map(|tg| tg.requests.len()).sum(),
            })
            .collect();
        // Stable sort by name so the list order is deterministic.
        summaries.sort_by(|a, b| a.name.cmp(&b.name));
        summaries
    }

    pub fn get_active_plan(&self) -> Option<&TestPlan> {
        self.active_plan_id.as_ref().and_then(|id| self.plans.get(id))
    }

    pub fn set_active_plan(&mut self, id: Uuid) {
        self.active_plan_id = Some(id);
    }

    /// Insert an already-constructed plan (e.g. loaded from disk) into the
    /// manager without generating a new ID.
    pub fn add_plan(&mut self, plan: TestPlan) {
        self.plans.insert(plan.id, plan);
    }

    // -----------------------------------------------------------------------
    // Thread Group operations
    // -----------------------------------------------------------------------

    /// Add a new default thread group to the specified plan.
    ///
    /// Returns the newly generated [`Uuid`] for the thread group.
    pub fn add_thread_group(
        &mut self,
        plan_id: &Uuid,
        name: String,
    ) -> Result<Uuid, RmeterError> {
        let plan = self
            .plans
            .get_mut(plan_id)
            .ok_or_else(|| RmeterError::PlanNotFound(plan_id.to_string()))?;

        let tg = ThreadGroup {
            id: Uuid::new_v4(),
            name,
            num_threads: 1,
            ramp_up_seconds: 0,
            loop_count: LoopCount::default(),
            requests: Vec::new(),
            enabled: true,
        };
        let id = tg.id;
        plan.thread_groups.push(tg);
        Ok(id)
    }

    /// Remove a thread group by ID from the specified plan.
    pub fn remove_thread_group(
        &mut self,
        plan_id: &Uuid,
        group_id: &Uuid,
    ) -> Result<(), RmeterError> {
        let plan = self
            .plans
            .get_mut(plan_id)
            .ok_or_else(|| RmeterError::PlanNotFound(plan_id.to_string()))?;

        let before = plan.thread_groups.len();
        plan.thread_groups.retain(|tg| &tg.id != group_id);
        if plan.thread_groups.len() == before {
            return Err(RmeterError::PlanNotFound(format!(
                "ThreadGroup {} not found in plan {}",
                group_id, plan_id
            )));
        }
        Ok(())
    }

    /// Apply a partial update to a thread group.
    ///
    /// Returns a reference to the updated group.
    pub fn update_thread_group(
        &mut self,
        plan_id: &Uuid,
        group_id: &Uuid,
        update: ThreadGroupUpdate,
    ) -> Result<&ThreadGroup, RmeterError> {
        let plan = self
            .plans
            .get_mut(plan_id)
            .ok_or_else(|| RmeterError::PlanNotFound(plan_id.to_string()))?;

        let tg = plan
            .thread_groups
            .iter_mut()
            .find(|tg| &tg.id == group_id)
            .ok_or_else(|| {
                RmeterError::PlanNotFound(format!(
                    "ThreadGroup {} not found in plan {}",
                    group_id, plan_id
                ))
            })?;

        if let Some(name) = update.name {
            tg.name = name;
        }
        if let Some(n) = update.num_threads {
            tg.num_threads = n;
        }
        if let Some(r) = update.ramp_up_seconds {
            tg.ramp_up_seconds = r;
        }
        if let Some(lc) = update.loop_count {
            tg.loop_count = lc;
        }
        if let Some(en) = update.enabled {
            tg.enabled = en;
        }

        // Re-borrow immutably to return a reference.
        let plan = self
            .plans
            .get(plan_id)
            .ok_or_else(|| RmeterError::PlanNotFound(plan_id.to_string()))?;
        let tg = plan
            .thread_groups
            .iter()
            .find(|tg| &tg.id == group_id)
            .ok_or_else(|| {
                RmeterError::Validation(format!("Thread group {} not found after update", group_id))
            })?;
        Ok(tg)
    }

    // -----------------------------------------------------------------------
    // Request operations
    // -----------------------------------------------------------------------

    /// Add a new default GET request to the specified thread group.
    ///
    /// Returns the newly generated [`Uuid`] for the request.
    pub fn add_request(
        &mut self,
        plan_id: &Uuid,
        group_id: &Uuid,
        name: String,
    ) -> Result<Uuid, RmeterError> {
        let plan = self
            .plans
            .get_mut(plan_id)
            .ok_or_else(|| RmeterError::PlanNotFound(plan_id.to_string()))?;

        let tg = plan
            .thread_groups
            .iter_mut()
            .find(|tg| &tg.id == group_id)
            .ok_or_else(|| {
                RmeterError::PlanNotFound(format!(
                    "ThreadGroup {} not found in plan {}",
                    group_id, plan_id
                ))
            })?;

        let req = HttpRequest {
            id: Uuid::new_v4(),
            name,
            method: HttpMethod::Get,
            url: String::new(),
            headers: HashMap::new(),
            body: None,
            assertions: Vec::new(),
            extractors: Vec::new(),
            enabled: true,
        };
        let id = req.id;
        tg.requests.push(req);
        Ok(id)
    }

    /// Remove a request from the specified thread group.
    pub fn remove_request(
        &mut self,
        plan_id: &Uuid,
        group_id: &Uuid,
        request_id: &Uuid,
    ) -> Result<(), RmeterError> {
        let plan = self
            .plans
            .get_mut(plan_id)
            .ok_or_else(|| RmeterError::PlanNotFound(plan_id.to_string()))?;

        let tg = plan
            .thread_groups
            .iter_mut()
            .find(|tg| &tg.id == group_id)
            .ok_or_else(|| {
                RmeterError::PlanNotFound(format!(
                    "ThreadGroup {} not found in plan {}",
                    group_id, plan_id
                ))
            })?;

        let before = tg.requests.len();
        tg.requests.retain(|r| &r.id != request_id);
        if tg.requests.len() == before {
            return Err(RmeterError::PlanNotFound(format!(
                "Request {} not found in thread group {}",
                request_id, group_id
            )));
        }
        Ok(())
    }

    /// Apply a partial update to a request.
    ///
    /// Returns a reference to the updated request.
    pub fn update_request(
        &mut self,
        plan_id: &Uuid,
        group_id: &Uuid,
        request_id: &Uuid,
        update: HttpRequestUpdate,
    ) -> Result<&HttpRequest, RmeterError> {
        let plan = self
            .plans
            .get_mut(plan_id)
            .ok_or_else(|| RmeterError::PlanNotFound(plan_id.to_string()))?;

        let tg = plan
            .thread_groups
            .iter_mut()
            .find(|tg| &tg.id == group_id)
            .ok_or_else(|| {
                RmeterError::PlanNotFound(format!(
                    "ThreadGroup {} not found in plan {}",
                    group_id, plan_id
                ))
            })?;

        let req = tg
            .requests
            .iter_mut()
            .find(|r| &r.id == request_id)
            .ok_or_else(|| {
                RmeterError::PlanNotFound(format!(
                    "Request {} not found in thread group {}",
                    request_id, group_id
                ))
            })?;

        if let Some(name) = update.name {
            req.name = name;
        }
        if let Some(method) = update.method {
            req.method = method;
        }
        if let Some(url) = update.url {
            req.url = url;
        }
        if let Some(headers) = update.headers {
            req.headers = headers;
        }
        if let Some(body) = update.body {
            req.body = body;
        }
        if let Some(en) = update.enabled {
            req.enabled = en;
        }

        // Re-borrow immutably to return a reference.
        let plan = self
            .plans
            .get(plan_id)
            .ok_or_else(|| RmeterError::PlanNotFound(plan_id.to_string()))?;
        let tg = plan
            .thread_groups
            .iter()
            .find(|tg| &tg.id == group_id)
            .ok_or_else(|| {
                RmeterError::Validation(format!("Thread group {} not found after update", group_id))
            })?;
        let req = tg
            .requests
            .iter()
            .find(|r| &r.id == request_id)
            .ok_or_else(|| {
                RmeterError::Validation(format!("Request {} not found after update", request_id))
            })?;
        Ok(req)
    }

    // -----------------------------------------------------------------------
    // Utilities
    // -----------------------------------------------------------------------

    /// Duplicate a thread group within the same plan.
    ///
    /// All contained requests receive new IDs as well.
    /// Returns the new thread group's ID.
    pub fn duplicate_thread_group(
        &mut self,
        plan_id: &Uuid,
        group_id: &Uuid,
    ) -> Result<Uuid, RmeterError> {
        let plan = self
            .plans
            .get_mut(plan_id)
            .ok_or_else(|| RmeterError::PlanNotFound(plan_id.to_string()))?;

        let original = plan
            .thread_groups
            .iter()
            .find(|tg| &tg.id == group_id)
            .ok_or_else(|| {
                RmeterError::PlanNotFound(format!(
                    "ThreadGroup {} not found in plan {}",
                    group_id, plan_id
                ))
            })?
            .clone();

        let mut copy = original;
        copy.id = Uuid::new_v4();
        copy.name = format!("{} (copy)", copy.name);
        for req in copy.requests.iter_mut() {
            req.id = Uuid::new_v4();
        }
        let new_id = copy.id;
        plan.thread_groups.push(copy);
        Ok(new_id)
    }

    /// Duplicate a request within the same thread group.
    ///
    /// Returns the new request's ID.
    pub fn duplicate_request(
        &mut self,
        plan_id: &Uuid,
        group_id: &Uuid,
        request_id: &Uuid,
    ) -> Result<Uuid, RmeterError> {
        let plan = self
            .plans
            .get_mut(plan_id)
            .ok_or_else(|| RmeterError::PlanNotFound(plan_id.to_string()))?;

        let tg = plan
            .thread_groups
            .iter_mut()
            .find(|tg| &tg.id == group_id)
            .ok_or_else(|| {
                RmeterError::PlanNotFound(format!(
                    "ThreadGroup {} not found in plan {}",
                    group_id, plan_id
                ))
            })?;

        let original = tg
            .requests
            .iter()
            .find(|r| &r.id == request_id)
            .ok_or_else(|| {
                RmeterError::PlanNotFound(format!(
                    "Request {} not found in thread group {}",
                    request_id, group_id
                ))
            })?
            .clone();

        let mut copy = original;
        copy.id = Uuid::new_v4();
        copy.name = format!("{} (copy)", copy.name);
        let new_id = copy.id;
        tg.requests.push(copy);
        Ok(new_id)
    }

    /// Reorder thread groups within a plan.
    ///
    /// `group_ids` must contain every thread group ID exactly once.
    pub fn reorder_thread_groups(
        &mut self,
        plan_id: &Uuid,
        group_ids: Vec<Uuid>,
    ) -> Result<(), RmeterError> {
        let plan = self
            .plans
            .get_mut(plan_id)
            .ok_or_else(|| RmeterError::PlanNotFound(plan_id.to_string()))?;

        if group_ids.len() != plan.thread_groups.len() {
            return Err(RmeterError::Validation(format!(
                "reorder_thread_groups: expected {} IDs, got {}",
                plan.thread_groups.len(),
                group_ids.len()
            )));
        }

        let mut reordered: Vec<ThreadGroup> = Vec::with_capacity(plan.thread_groups.len());
        for id in &group_ids {
            let tg = plan
                .thread_groups
                .iter()
                .find(|tg| &tg.id == id)
                .ok_or_else(|| {
                    RmeterError::PlanNotFound(format!(
                        "ThreadGroup {} not found in plan {}",
                        id, plan_id
                    ))
                })?
                .clone();
            reordered.push(tg);
        }
        plan.thread_groups = reordered;
        Ok(())
    }

    /// Reorder requests within a thread group.
    ///
    /// `request_ids` must contain every request ID exactly once.
    pub fn reorder_requests(
        &mut self,
        plan_id: &Uuid,
        group_id: &Uuid,
        request_ids: Vec<Uuid>,
    ) -> Result<(), RmeterError> {
        let plan = self
            .plans
            .get_mut(plan_id)
            .ok_or_else(|| RmeterError::PlanNotFound(plan_id.to_string()))?;

        let tg = plan
            .thread_groups
            .iter_mut()
            .find(|tg| &tg.id == group_id)
            .ok_or_else(|| {
                RmeterError::PlanNotFound(format!(
                    "ThreadGroup {} not found in plan {}",
                    group_id, plan_id
                ))
            })?;

        if request_ids.len() != tg.requests.len() {
            return Err(RmeterError::Validation(format!(
                "reorder_requests: expected {} IDs, got {}",
                tg.requests.len(),
                request_ids.len()
            )));
        }

        let mut reordered: Vec<HttpRequest> = Vec::with_capacity(tg.requests.len());
        for id in &request_ids {
            let req = tg
                .requests
                .iter()
                .find(|r| &r.id == id)
                .ok_or_else(|| {
                    RmeterError::PlanNotFound(format!(
                        "Request {} not found in thread group {}",
                        id, group_id
                    ))
                })?
                .clone();
            reordered.push(req);
        }
        tg.requests = reordered;
        Ok(())
    }

    /// Toggle the `enabled` flag on either a thread group or a request.
    ///
    /// Searches thread groups first, then requests inside all thread groups.
    /// Returns the new enabled state.
    pub fn toggle_enabled(
        &mut self,
        plan_id: &Uuid,
        element_id: &Uuid,
    ) -> Result<bool, RmeterError> {
        let plan = self
            .plans
            .get_mut(plan_id)
            .ok_or_else(|| RmeterError::PlanNotFound(plan_id.to_string()))?;

        // Check thread groups first.
        for tg in plan.thread_groups.iter_mut() {
            if &tg.id == element_id {
                tg.enabled = !tg.enabled;
                return Ok(tg.enabled);
            }
            // Check requests within this thread group.
            for req in tg.requests.iter_mut() {
                if &req.id == element_id {
                    req.enabled = !req.enabled;
                    return Ok(req.enabled);
                }
            }
        }

        Err(RmeterError::PlanNotFound(format!(
            "Element {} not found in plan {}",
            element_id, plan_id
        )))
    }

    /// Rename a thread group or request.
    ///
    /// Searches thread groups first, then requests inside all thread groups.
    pub fn rename_element(
        &mut self,
        plan_id: &Uuid,
        element_id: &Uuid,
        new_name: String,
    ) -> Result<(), RmeterError> {
        let plan = self
            .plans
            .get_mut(plan_id)
            .ok_or_else(|| RmeterError::PlanNotFound(plan_id.to_string()))?;

        // Check thread groups first.
        for tg in plan.thread_groups.iter_mut() {
            if &tg.id == element_id {
                tg.name = new_name;
                return Ok(());
            }
            for req in tg.requests.iter_mut() {
                if &req.id == element_id {
                    req.name = new_name;
                    return Ok(());
                }
            }
        }

        Err(RmeterError::PlanNotFound(format!(
            "Element {} not found in plan {}",
            element_id, plan_id
        )))
    }

    // -----------------------------------------------------------------------
    // Assertion operations
    // -----------------------------------------------------------------------

    /// Add an [`Assertion`] to the specified request.
    ///
    /// Returns the newly created assertion.
    pub fn add_assertion(
        &mut self,
        plan_id: &Uuid,
        group_id: &Uuid,
        request_id: &Uuid,
        name: String,
        rule: serde_json::Value,
    ) -> Result<Assertion, RmeterError> {
        let req = self.find_request_mut(plan_id, group_id, request_id)?;

        let assertion = Assertion {
            id: Uuid::new_v4(),
            name,
            rule,
        };
        let created = assertion.clone();
        req.assertions.push(assertion);
        Ok(created)
    }

    /// Remove an assertion from the specified request.
    pub fn remove_assertion(
        &mut self,
        plan_id: &Uuid,
        group_id: &Uuid,
        request_id: &Uuid,
        assertion_id: &Uuid,
    ) -> Result<(), RmeterError> {
        let req = self.find_request_mut(plan_id, group_id, request_id)?;

        let before = req.assertions.len();
        req.assertions.retain(|a| &a.id != assertion_id);
        if req.assertions.len() == before {
            return Err(RmeterError::PlanNotFound(format!(
                "Assertion {} not found in request {}",
                assertion_id, request_id
            )));
        }
        Ok(())
    }

    /// Apply a partial update to an assertion and return the updated assertion.
    pub fn update_assertion(
        &mut self,
        plan_id: &Uuid,
        group_id: &Uuid,
        request_id: &Uuid,
        assertion_id: &Uuid,
        name: Option<String>,
        rule: Option<serde_json::Value>,
    ) -> Result<Assertion, RmeterError> {
        let req = self.find_request_mut(plan_id, group_id, request_id)?;

        let assertion = req
            .assertions
            .iter_mut()
            .find(|a| &a.id == assertion_id)
            .ok_or_else(|| {
                RmeterError::PlanNotFound(format!(
                    "Assertion {} not found in request {}",
                    assertion_id, request_id
                ))
            })?;

        if let Some(n) = name {
            assertion.name = n;
        }
        if let Some(r) = rule {
            assertion.rule = r;
        }

        Ok(assertion.clone())
    }

    // -----------------------------------------------------------------------
    // Variable operations
    // -----------------------------------------------------------------------

    /// Add a [`Variable`] to the specified plan and return the created variable.
    pub fn add_variable(
        &mut self,
        plan_id: &Uuid,
        name: String,
        value: String,
        scope: VariableScope,
    ) -> Result<Variable, RmeterError> {
        let plan = self
            .plans
            .get_mut(plan_id)
            .ok_or_else(|| RmeterError::PlanNotFound(plan_id.to_string()))?;

        let variable = Variable {
            id: Uuid::new_v4(),
            name,
            value,
            scope,
        };
        let created = variable.clone();
        plan.variables.push(variable);
        Ok(created)
    }

    /// Remove a variable from the specified plan by ID.
    pub fn remove_variable(
        &mut self,
        plan_id: &Uuid,
        variable_id: &Uuid,
    ) -> Result<(), RmeterError> {
        let plan = self
            .plans
            .get_mut(plan_id)
            .ok_or_else(|| RmeterError::PlanNotFound(plan_id.to_string()))?;

        let before = plan.variables.len();
        plan.variables.retain(|v| &v.id != variable_id);
        if plan.variables.len() == before {
            return Err(RmeterError::PlanNotFound(format!(
                "Variable {} not found in plan {}",
                variable_id, plan_id
            )));
        }
        Ok(())
    }

    /// Apply a partial update to a variable and return the updated variable.
    pub fn update_variable(
        &mut self,
        plan_id: &Uuid,
        variable_id: &Uuid,
        name: Option<String>,
        value: Option<String>,
        scope: Option<VariableScope>,
    ) -> Result<Variable, RmeterError> {
        let plan = self
            .plans
            .get_mut(plan_id)
            .ok_or_else(|| RmeterError::PlanNotFound(plan_id.to_string()))?;

        let variable = plan
            .variables
            .iter_mut()
            .find(|v| &v.id == variable_id)
            .ok_or_else(|| {
                RmeterError::PlanNotFound(format!(
                    "Variable {} not found in plan {}",
                    variable_id, plan_id
                ))
            })?;

        if let Some(n) = name {
            variable.name = n;
        }
        if let Some(v) = value {
            variable.value = v;
        }
        if let Some(s) = scope {
            variable.scope = s;
        }

        Ok(variable.clone())
    }

    // -----------------------------------------------------------------------
    // Extractor operations
    // -----------------------------------------------------------------------

    /// Add an [`Extractor`] to the specified request and return the created extractor.
    pub fn add_extractor(
        &mut self,
        plan_id: &Uuid,
        group_id: &Uuid,
        request_id: &Uuid,
        name: String,
        variable: String,
        expression: serde_json::Value,
    ) -> Result<Extractor, RmeterError> {
        let req = self.find_request_mut(plan_id, group_id, request_id)?;

        let extractor = Extractor {
            id: Uuid::new_v4(),
            name,
            variable,
            expression,
        };
        let created = extractor.clone();
        req.extractors.push(extractor);
        Ok(created)
    }

    /// Remove an extractor from the specified request.
    pub fn remove_extractor(
        &mut self,
        plan_id: &Uuid,
        group_id: &Uuid,
        request_id: &Uuid,
        extractor_id: &Uuid,
    ) -> Result<(), RmeterError> {
        let req = self.find_request_mut(plan_id, group_id, request_id)?;

        let before = req.extractors.len();
        req.extractors.retain(|e| &e.id != extractor_id);
        if req.extractors.len() == before {
            return Err(RmeterError::PlanNotFound(format!(
                "Extractor {} not found in request {}",
                extractor_id, request_id
            )));
        }
        Ok(())
    }

    /// Apply a partial update to an extractor and return the updated extractor.
    #[allow(clippy::too_many_arguments)]
    pub fn update_extractor(
        &mut self,
        plan_id: &Uuid,
        group_id: &Uuid,
        request_id: &Uuid,
        extractor_id: &Uuid,
        name: Option<String>,
        variable: Option<String>,
        expression: Option<serde_json::Value>,
    ) -> Result<Extractor, RmeterError> {
        let req = self.find_request_mut(plan_id, group_id, request_id)?;

        let extractor = req
            .extractors
            .iter_mut()
            .find(|e| &e.id == extractor_id)
            .ok_or_else(|| {
                RmeterError::PlanNotFound(format!(
                    "Extractor {} not found in request {}",
                    extractor_id, request_id
                ))
            })?;

        if let Some(n) = name {
            extractor.name = n;
        }
        if let Some(v) = variable {
            extractor.variable = v;
        }
        if let Some(e) = expression {
            extractor.expression = e;
        }

        Ok(extractor.clone())
    }

    // -----------------------------------------------------------------------
    // Private navigation helpers
    // -----------------------------------------------------------------------

    /// Find a mutable reference to an [`HttpRequest`] by navigating through
    /// plan -> thread group -> request.
    fn find_request_mut(
        &mut self,
        plan_id: &Uuid,
        group_id: &Uuid,
        request_id: &Uuid,
    ) -> Result<&mut HttpRequest, RmeterError> {
        let plan = self
            .plans
            .get_mut(plan_id)
            .ok_or_else(|| RmeterError::PlanNotFound(plan_id.to_string()))?;

        let tg = plan
            .thread_groups
            .iter_mut()
            .find(|tg| &tg.id == group_id)
            .ok_or_else(|| {
                RmeterError::PlanNotFound(format!(
                    "ThreadGroup {} not found in plan {}",
                    group_id, plan_id
                ))
            })?;

        tg.requests
            .iter_mut()
            .find(|r| &r.id == request_id)
            .ok_or_else(|| {
                RmeterError::PlanNotFound(format!(
                    "Request {} not found in thread group {}",
                    request_id, group_id
                ))
            })
    }

    // -----------------------------------------------------------------------
    // CSV Data Source operations
    // -----------------------------------------------------------------------

    /// Add a CSV data source by parsing raw CSV content (with header row).
    pub fn add_csv_data_source(
        &mut self,
        plan_id: &Uuid,
        name: String,
        csv_content: String,
        delimiter: Option<u8>,
    ) -> Result<crate::plan::model::CsvDataSource, RmeterError> {
        let plan = self
            .plans
            .get_mut(plan_id)
            .ok_or_else(|| RmeterError::PlanNotFound(plan_id.to_string()))?;

        let source = crate::plan::model::CsvDataSource::from_csv_content(
            name,
            &csv_content,
            delimiter.unwrap_or(b','),
        )
        .map_err(RmeterError::Validation)?;

        let created = source.clone();
        plan.csv_data_sources.push(source);
        Ok(created)
    }

    /// Remove a CSV data source from the specified plan by ID.
    pub fn remove_csv_data_source(
        &mut self,
        plan_id: &Uuid,
        source_id: &Uuid,
    ) -> Result<(), RmeterError> {
        let plan = self
            .plans
            .get_mut(plan_id)
            .ok_or_else(|| RmeterError::PlanNotFound(plan_id.to_string()))?;

        let before = plan.csv_data_sources.len();
        plan.csv_data_sources.retain(|s| s.id != *source_id);

        if plan.csv_data_sources.len() == before {
            return Err(RmeterError::PlanNotFound(format!(
                "CSV data source {} not found in plan {}",
                source_id, plan_id
            )));
        }
        Ok(())
    }

    /// Update a CSV data source's sharing mode or recycle flag.
    pub fn update_csv_data_source(
        &mut self,
        plan_id: &Uuid,
        source_id: &Uuid,
        name: Option<String>,
        sharing_mode: Option<crate::plan::model::CsvSharingMode>,
        recycle: Option<bool>,
    ) -> Result<&crate::plan::model::CsvDataSource, RmeterError> {
        let plan = self
            .plans
            .get_mut(plan_id)
            .ok_or_else(|| RmeterError::PlanNotFound(plan_id.to_string()))?;

        let source = plan
            .csv_data_sources
            .iter_mut()
            .find(|s| s.id == *source_id)
            .ok_or_else(|| {
                RmeterError::PlanNotFound(format!(
                    "CSV data source {} not found in plan {}",
                    source_id, plan_id
                ))
            })?;

        if let Some(n) = name {
            source.name = n;
        }
        if let Some(sm) = sharing_mode {
            source.sharing_mode = sm;
        }
        if let Some(r) = recycle {
            source.recycle = r;
        }

        Ok(source)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Helper: create a manager with one plan and return (manager, plan_id).
    fn manager_with_plan(name: &str) -> (PlanManager, Uuid) {
        let mut mgr = PlanManager::new();
        let id = mgr.create_plan(name.to_string());
        (mgr, id)
    }

    // Helper: create a manager with one plan + one thread group.
    fn manager_with_group(plan_name: &str, group_name: &str) -> (PlanManager, Uuid, Uuid) {
        let (mut mgr, plan_id) = manager_with_plan(plan_name);
        let group_id = mgr.add_thread_group(&plan_id, group_name.to_string()).unwrap();
        (mgr, plan_id, group_id)
    }

    // Helper: create a manager with one plan + one thread group + one request.
    fn manager_with_request(
        plan_name: &str,
        group_name: &str,
        req_name: &str,
    ) -> (PlanManager, Uuid, Uuid, Uuid) {
        let (mut mgr, plan_id, group_id) = manager_with_group(plan_name, group_name);
        let req_id = mgr.add_request(&plan_id, &group_id, req_name.to_string()).unwrap();
        (mgr, plan_id, group_id, req_id)
    }

    // -----------------------------------------------------------------------
    // Plan CRUD
    // -----------------------------------------------------------------------

    #[test]
    fn create_plan_returns_id_and_is_retrievable() {
        let (mgr, id) = manager_with_plan("My Plan");
        let plan = mgr.get_plan(&id).expect("plan should exist");
        assert_eq!(plan.name, "My Plan");
        assert_eq!(plan.id, id);
    }

    #[test]
    fn delete_plan_removes_plan() {
        let (mut mgr, id) = manager_with_plan("To Delete");
        let removed = mgr.delete_plan(&id);
        assert!(removed);
        assert!(mgr.get_plan(&id).is_none());
    }

    #[test]
    fn delete_plan_returns_false_for_unknown_id() {
        let mut mgr = PlanManager::new();
        let unknown = Uuid::new_v4();
        assert!(!mgr.delete_plan(&unknown));
    }

    #[test]
    fn delete_active_plan_clears_active() {
        let (mut mgr, id) = manager_with_plan("Active");
        mgr.set_active_plan(id);
        assert!(mgr.get_active_plan().is_some());
        mgr.delete_plan(&id);
        assert!(mgr.get_active_plan().is_none());
    }

    #[test]
    fn list_plans_is_sorted_by_name() {
        let mut mgr = PlanManager::new();
        mgr.create_plan("Zebra".to_string());
        mgr.create_plan("Alpha".to_string());
        mgr.create_plan("Middle".to_string());
        let summaries = mgr.list_plans();
        let names: Vec<&str> = summaries.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["Alpha", "Middle", "Zebra"]);
    }

    #[test]
    fn list_plans_counts_are_correct() {
        let (mut mgr, plan_id) = manager_with_plan("Counted");
        let g1 = mgr.add_thread_group(&plan_id, "G1".to_string()).unwrap();
        let g2 = mgr.add_thread_group(&plan_id, "G2".to_string()).unwrap();
        mgr.add_request(&plan_id, &g1, "R1".to_string()).unwrap();
        mgr.add_request(&plan_id, &g1, "R2".to_string()).unwrap();
        mgr.add_request(&plan_id, &g2, "R3".to_string()).unwrap();

        let summaries = mgr.list_plans();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].thread_group_count, 2);
        assert_eq!(summaries[0].request_count, 3);
    }

    #[test]
    fn set_and_get_active_plan() {
        let (mut mgr, id) = manager_with_plan("Active Plan");
        assert!(mgr.get_active_plan().is_none());
        mgr.set_active_plan(id);
        let active = mgr.get_active_plan().expect("active plan should be set");
        assert_eq!(active.id, id);
    }

    #[test]
    fn add_plan_inserts_without_new_id() {
        let mut mgr = PlanManager::new();
        let plan = crate::plan::model::TestPlan::new("Loaded Plan");
        let original_id = plan.id;
        mgr.add_plan(plan);
        let retrieved = mgr.get_plan(&original_id).expect("plan should be present");
        assert_eq!(retrieved.id, original_id);
    }

    // -----------------------------------------------------------------------
    // Thread Group operations
    // -----------------------------------------------------------------------

    #[test]
    fn add_thread_group_adds_to_plan() {
        let (mut mgr, plan_id) = manager_with_plan("Plan");
        let group_id = mgr.add_thread_group(&plan_id, "Workers".to_string()).unwrap();
        let plan = mgr.get_plan(&plan_id).unwrap();
        assert_eq!(plan.thread_groups.len(), 1);
        assert_eq!(plan.thread_groups[0].id, group_id);
        assert_eq!(plan.thread_groups[0].name, "Workers");
    }

    #[test]
    fn add_thread_group_error_for_missing_plan() {
        let mut mgr = PlanManager::new();
        let unknown = Uuid::new_v4();
        let result = mgr.add_thread_group(&unknown, "G".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn remove_thread_group_removes_it() {
        let (mut mgr, plan_id, group_id) = manager_with_group("Plan", "G");
        mgr.remove_thread_group(&plan_id, &group_id).unwrap();
        let plan = mgr.get_plan(&plan_id).unwrap();
        assert!(plan.thread_groups.is_empty());
    }

    #[test]
    fn remove_thread_group_error_for_missing_group() {
        let (mut mgr, plan_id) = manager_with_plan("Plan");
        let unknown = Uuid::new_v4();
        let result = mgr.remove_thread_group(&plan_id, &unknown);
        assert!(result.is_err());
    }

    #[test]
    fn update_thread_group_applies_partial_fields() {
        let (mut mgr, plan_id, group_id) = manager_with_group("Plan", "Original Name");
        let update = ThreadGroupUpdate {
            name: Some("New Name".to_string()),
            num_threads: Some(5),
            ramp_up_seconds: None,
            loop_count: None,
            enabled: None,
        };
        let updated = mgr.update_thread_group(&plan_id, &group_id, update).unwrap();
        assert_eq!(updated.name, "New Name");
        assert_eq!(updated.num_threads, 5);
        // ramp_up_seconds should remain at default (0)
        assert_eq!(updated.ramp_up_seconds, 0);
    }

    #[test]
    fn update_thread_group_loop_count() {
        let (mut mgr, plan_id, group_id) = manager_with_group("Plan", "G");
        let update = ThreadGroupUpdate {
            name: None,
            num_threads: None,
            ramp_up_seconds: None,
            loop_count: Some(LoopCount::Duration { seconds: 60 }),
            enabled: None,
        };
        let updated = mgr.update_thread_group(&plan_id, &group_id, update).unwrap();
        assert!(matches!(updated.loop_count, LoopCount::Duration { seconds: 60 }));
    }

    // -----------------------------------------------------------------------
    // Request operations
    // -----------------------------------------------------------------------

    #[test]
    fn add_request_adds_to_thread_group() {
        let (mut mgr, plan_id, group_id) = manager_with_group("Plan", "G");
        let req_id = mgr.add_request(&plan_id, &group_id, "Login".to_string()).unwrap();
        let plan = mgr.get_plan(&plan_id).unwrap();
        let tg = &plan.thread_groups[0];
        assert_eq!(tg.requests.len(), 1);
        assert_eq!(tg.requests[0].id, req_id);
        assert_eq!(tg.requests[0].name, "Login");
        assert_eq!(tg.requests[0].method, HttpMethod::Get);
    }

    #[test]
    fn add_request_error_for_missing_plan() {
        let mut mgr = PlanManager::new();
        let unknown_plan = Uuid::new_v4();
        let unknown_group = Uuid::new_v4();
        let result = mgr.add_request(&unknown_plan, &unknown_group, "R".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn add_request_error_for_missing_group() {
        let (mut mgr, plan_id) = manager_with_plan("Plan");
        let unknown_group = Uuid::new_v4();
        let result = mgr.add_request(&plan_id, &unknown_group, "R".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn remove_request_removes_it() {
        let (mut mgr, plan_id, group_id, req_id) =
            manager_with_request("Plan", "G", "Request");
        mgr.remove_request(&plan_id, &group_id, &req_id).unwrap();
        let plan = mgr.get_plan(&plan_id).unwrap();
        assert!(plan.thread_groups[0].requests.is_empty());
    }

    #[test]
    fn remove_request_error_for_missing_request() {
        let (mut mgr, plan_id, group_id) = manager_with_group("Plan", "G");
        let unknown = Uuid::new_v4();
        let result = mgr.remove_request(&plan_id, &group_id, &unknown);
        assert!(result.is_err());
    }

    #[test]
    fn update_request_applies_partial_fields() {
        let (mut mgr, plan_id, group_id, req_id) =
            manager_with_request("Plan", "G", "Original");
        let update = HttpRequestUpdate {
            name: Some("Updated".to_string()),
            method: Some(HttpMethod::Post),
            url: Some("https://example.com".to_string()),
            headers: None,
            body: None,
            enabled: None,
        };
        let updated = mgr.update_request(&plan_id, &group_id, &req_id, update).unwrap();
        assert_eq!(updated.name, "Updated");
        assert_eq!(updated.method, HttpMethod::Post);
        assert_eq!(updated.url, "https://example.com");
        // enabled should remain at default (true)
        assert!(updated.enabled);
    }

    #[test]
    fn update_request_clears_body_with_some_none() {
        let (mut mgr, plan_id, group_id, req_id) =
            manager_with_request("Plan", "G", "R");
        // First set a body.
        let set_body = HttpRequestUpdate {
            name: None,
            method: None,
            url: None,
            headers: None,
            body: Some(Some(RequestBody::Raw("hello".to_string()))),
            enabled: None,
        };
        mgr.update_request(&plan_id, &group_id, &req_id, set_body).unwrap();

        // Now clear it.
        let clear_body = HttpRequestUpdate {
            name: None,
            method: None,
            url: None,
            headers: None,
            body: Some(None),
            enabled: None,
        };
        let updated = mgr.update_request(&plan_id, &group_id, &req_id, clear_body).unwrap();
        assert!(updated.body.is_none());
    }

    // -----------------------------------------------------------------------
    // Duplicate operations
    // -----------------------------------------------------------------------

    #[test]
    fn duplicate_thread_group_creates_copy_with_new_id() {
        let (mut mgr, plan_id, group_id) = manager_with_group("Plan", "Original");
        // Add a request to the original so we can verify it gets a new ID.
        mgr.add_request(&plan_id, &group_id, "Req".to_string()).unwrap();

        let new_id = mgr.duplicate_thread_group(&plan_id, &group_id).unwrap();
        assert_ne!(new_id, group_id);

        let plan = mgr.get_plan(&plan_id).unwrap();
        let copy = plan.thread_groups.iter().find(|tg| tg.id == new_id).unwrap();
        assert_eq!(copy.name, "Original (copy)");
        // The request inside the copy should have a different ID than the original.
        let original = plan.thread_groups.iter().find(|tg| tg.id == group_id).unwrap();
        assert_ne!(copy.requests[0].id, original.requests[0].id);
    }

    #[test]
    fn duplicate_thread_group_error_for_missing_group() {
        let (mut mgr, plan_id) = manager_with_plan("Plan");
        let unknown = Uuid::new_v4();
        let result = mgr.duplicate_thread_group(&plan_id, &unknown);
        assert!(result.is_err());
    }

    #[test]
    fn duplicate_request_creates_copy_with_new_id() {
        let (mut mgr, plan_id, group_id, req_id) =
            manager_with_request("Plan", "G", "Original Request");
        let new_id = mgr.duplicate_request(&plan_id, &group_id, &req_id).unwrap();
        assert_ne!(new_id, req_id);

        let plan = mgr.get_plan(&plan_id).unwrap();
        let tg = &plan.thread_groups[0];
        assert_eq!(tg.requests.len(), 2);
        let copy = tg.requests.iter().find(|r| r.id == new_id).unwrap();
        assert_eq!(copy.name, "Original Request (copy)");
    }

    #[test]
    fn duplicate_request_error_for_missing_request() {
        let (mut mgr, plan_id, group_id) = manager_with_group("Plan", "G");
        let unknown = Uuid::new_v4();
        let result = mgr.duplicate_request(&plan_id, &group_id, &unknown);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Reorder operations
    // -----------------------------------------------------------------------

    #[test]
    fn reorder_thread_groups_correct_order() {
        let (mut mgr, plan_id) = manager_with_plan("Plan");
        let g1 = mgr.add_thread_group(&plan_id, "First".to_string()).unwrap();
        let g2 = mgr.add_thread_group(&plan_id, "Second".to_string()).unwrap();
        let g3 = mgr.add_thread_group(&plan_id, "Third".to_string()).unwrap();

        mgr.reorder_thread_groups(&plan_id, vec![g3, g1, g2]).unwrap();

        let plan = mgr.get_plan(&plan_id).unwrap();
        assert_eq!(plan.thread_groups[0].id, g3);
        assert_eq!(plan.thread_groups[1].id, g1);
        assert_eq!(plan.thread_groups[2].id, g2);
    }

    #[test]
    fn reorder_thread_groups_error_on_wrong_count() {
        let (mut mgr, plan_id) = manager_with_plan("Plan");
        let g1 = mgr.add_thread_group(&plan_id, "G1".to_string()).unwrap();
        let _g2 = mgr.add_thread_group(&plan_id, "G2".to_string()).unwrap();

        // Only provide one ID for a plan with two groups.
        let result = mgr.reorder_thread_groups(&plan_id, vec![g1]);
        assert!(result.is_err());
    }

    #[test]
    fn reorder_requests_correct_order() {
        let (mut mgr, plan_id, group_id) = manager_with_group("Plan", "G");
        let r1 = mgr.add_request(&plan_id, &group_id, "R1".to_string()).unwrap();
        let r2 = mgr.add_request(&plan_id, &group_id, "R2".to_string()).unwrap();
        let r3 = mgr.add_request(&plan_id, &group_id, "R3".to_string()).unwrap();

        mgr.reorder_requests(&plan_id, &group_id, vec![r3, r1, r2]).unwrap();

        let plan = mgr.get_plan(&plan_id).unwrap();
        let reqs = &plan.thread_groups[0].requests;
        assert_eq!(reqs[0].id, r3);
        assert_eq!(reqs[1].id, r1);
        assert_eq!(reqs[2].id, r2);
    }

    #[test]
    fn reorder_requests_error_on_wrong_count() {
        let (mut mgr, plan_id, group_id) = manager_with_group("Plan", "G");
        let r1 = mgr.add_request(&plan_id, &group_id, "R1".to_string()).unwrap();
        let _r2 = mgr.add_request(&plan_id, &group_id, "R2".to_string()).unwrap();

        let result = mgr.reorder_requests(&plan_id, &group_id, vec![r1]);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Toggle / Rename
    // -----------------------------------------------------------------------

    #[test]
    fn toggle_enabled_thread_group() {
        let (mut mgr, plan_id, group_id) = manager_with_group("Plan", "G");
        // Default is enabled=true.
        let new_state = mgr.toggle_enabled(&plan_id, &group_id).unwrap();
        assert!(!new_state);
        // Toggle again.
        let new_state = mgr.toggle_enabled(&plan_id, &group_id).unwrap();
        assert!(new_state);
    }

    #[test]
    fn toggle_enabled_request() {
        let (mut mgr, plan_id, _group_id, req_id) =
            manager_with_request("Plan", "G", "Req");
        let new_state = mgr.toggle_enabled(&plan_id, &req_id).unwrap();
        assert!(!new_state);
    }

    #[test]
    fn toggle_enabled_error_for_unknown_element() {
        let (mut mgr, plan_id) = manager_with_plan("Plan");
        let unknown = Uuid::new_v4();
        let result = mgr.toggle_enabled(&plan_id, &unknown);
        assert!(result.is_err());
    }

    #[test]
    fn rename_element_renames_thread_group() {
        let (mut mgr, plan_id, group_id) = manager_with_group("Plan", "Old Name");
        mgr.rename_element(&plan_id, &group_id, "New Name".to_string()).unwrap();
        let plan = mgr.get_plan(&plan_id).unwrap();
        assert_eq!(plan.thread_groups[0].name, "New Name");
    }

    #[test]
    fn rename_element_renames_request() {
        let (mut mgr, plan_id, _group_id, req_id) =
            manager_with_request("Plan", "G", "Old Req");
        mgr.rename_element(&plan_id, &req_id, "New Req".to_string()).unwrap();
        let plan = mgr.get_plan(&plan_id).unwrap();
        assert_eq!(plan.thread_groups[0].requests[0].name, "New Req");
    }

    #[test]
    fn rename_element_error_for_unknown_element() {
        let (mut mgr, plan_id) = manager_with_plan("Plan");
        let unknown = Uuid::new_v4();
        let result = mgr.rename_element(&plan_id, &unknown, "Name".to_string());
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Variable CRUD
    // -----------------------------------------------------------------------

    #[test]
    fn add_variable_creates_variable() {
        let (mut mgr, plan_id) = manager_with_plan("Plan");
        let var = mgr
            .add_variable(&plan_id, "BASE_URL".to_string(), "https://example.com".to_string(), VariableScope::Plan)
            .unwrap();
        assert_eq!(var.name, "BASE_URL");
        assert_eq!(var.value, "https://example.com");

        let plan = mgr.get_plan(&plan_id).unwrap();
        assert_eq!(plan.variables.len(), 1);
        assert_eq!(plan.variables[0].id, var.id);
    }

    #[test]
    fn remove_variable_removes_it() {
        let (mut mgr, plan_id) = manager_with_plan("Plan");
        let var = mgr
            .add_variable(&plan_id, "TOKEN".to_string(), "abc123".to_string(), VariableScope::Global)
            .unwrap();
        mgr.remove_variable(&plan_id, &var.id).unwrap();
        let plan = mgr.get_plan(&plan_id).unwrap();
        assert!(plan.variables.is_empty());
    }

    #[test]
    fn remove_variable_error_for_missing_variable() {
        let (mut mgr, plan_id) = manager_with_plan("Plan");
        let unknown = Uuid::new_v4();
        let result = mgr.remove_variable(&plan_id, &unknown);
        assert!(result.is_err());
    }

    #[test]
    fn update_variable_applies_partial_fields() {
        let (mut mgr, plan_id) = manager_with_plan("Plan");
        let var = mgr
            .add_variable(&plan_id, "OLD".to_string(), "old_val".to_string(), VariableScope::Plan)
            .unwrap();

        let updated = mgr
            .update_variable(&plan_id, &var.id, Some("NEW".to_string()), Some("new_val".to_string()), None)
            .unwrap();
        assert_eq!(updated.name, "NEW");
        assert_eq!(updated.value, "new_val");
        // scope should remain Plan
        assert!(matches!(updated.scope, VariableScope::Plan));
    }

    #[test]
    fn update_variable_error_for_missing_variable() {
        let (mut mgr, plan_id) = manager_with_plan("Plan");
        let unknown = Uuid::new_v4();
        let result = mgr.update_variable(&plan_id, &unknown, None, None, None);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Extractor CRUD
    // -----------------------------------------------------------------------

    #[test]
    fn add_extractor_creates_extractor() {
        let (mut mgr, plan_id, group_id, req_id) =
            manager_with_request("Plan", "G", "R");
        let expr = serde_json::json!({"type": "json_path", "expression": "$.token"});
        let ext = mgr
            .add_extractor(&plan_id, &group_id, &req_id, "Extract Token".to_string(), "token".to_string(), expr.clone())
            .unwrap();
        assert_eq!(ext.name, "Extract Token");
        assert_eq!(ext.variable, "token");
        assert_eq!(ext.expression, expr);

        let plan = mgr.get_plan(&plan_id).unwrap();
        assert_eq!(plan.thread_groups[0].requests[0].extractors.len(), 1);
    }

    #[test]
    fn remove_extractor_removes_it() {
        let (mut mgr, plan_id, group_id, req_id) =
            manager_with_request("Plan", "G", "R");
        let expr = serde_json::json!({});
        let ext = mgr
            .add_extractor(&plan_id, &group_id, &req_id, "E".to_string(), "v".to_string(), expr)
            .unwrap();
        mgr.remove_extractor(&plan_id, &group_id, &req_id, &ext.id).unwrap();
        let plan = mgr.get_plan(&plan_id).unwrap();
        assert!(plan.thread_groups[0].requests[0].extractors.is_empty());
    }

    #[test]
    fn remove_extractor_error_for_missing_extractor() {
        let (mut mgr, plan_id, group_id, req_id) =
            manager_with_request("Plan", "G", "R");
        let unknown = Uuid::new_v4();
        let result = mgr.remove_extractor(&plan_id, &group_id, &req_id, &unknown);
        assert!(result.is_err());
    }

    #[test]
    fn update_extractor_applies_partial_fields() {
        let (mut mgr, plan_id, group_id, req_id) =
            manager_with_request("Plan", "G", "R");
        let expr = serde_json::json!({"type": "json_path", "expression": "$.old"});
        let ext = mgr
            .add_extractor(&plan_id, &group_id, &req_id, "Old Name".to_string(), "old_var".to_string(), expr)
            .unwrap();

        let new_expr = serde_json::json!({"type": "json_path", "expression": "$.new"});
        let updated = mgr
            .update_extractor(&plan_id, &group_id, &req_id, &ext.id, Some("New Name".to_string()), Some("new_var".to_string()), Some(new_expr.clone()))
            .unwrap();
        assert_eq!(updated.name, "New Name");
        assert_eq!(updated.variable, "new_var");
        assert_eq!(updated.expression, new_expr);
    }

    #[test]
    fn update_extractor_error_for_missing_extractor() {
        let (mut mgr, plan_id, group_id, req_id) =
            manager_with_request("Plan", "G", "R");
        let unknown = Uuid::new_v4();
        let result = mgr.update_extractor(&plan_id, &group_id, &req_id, &unknown, None, None, None);
        assert!(result.is_err());
    }

    // -----------------------------------------------------------------------
    // Assertion CRUD
    // -----------------------------------------------------------------------

    #[test]
    fn add_assertion_creates_assertion() {
        let (mut mgr, plan_id, group_id, req_id) =
            manager_with_request("Plan", "G", "R");
        let rule = serde_json::json!({"type": "status_code_equals", "expected": 200});
        let assertion = mgr
            .add_assertion(&plan_id, &group_id, &req_id, "Status 200".to_string(), rule.clone())
            .unwrap();
        assert_eq!(assertion.name, "Status 200");
        assert_eq!(assertion.rule, rule);

        let plan = mgr.get_plan(&plan_id).unwrap();
        assert_eq!(plan.thread_groups[0].requests[0].assertions.len(), 1);
    }

    #[test]
    fn remove_assertion_removes_it() {
        let (mut mgr, plan_id, group_id, req_id) =
            manager_with_request("Plan", "G", "R");
        let rule = serde_json::json!({"type": "status_code_equals", "expected": 200});
        let assertion = mgr
            .add_assertion(&plan_id, &group_id, &req_id, "A".to_string(), rule)
            .unwrap();
        mgr.remove_assertion(&plan_id, &group_id, &req_id, &assertion.id).unwrap();
        let plan = mgr.get_plan(&plan_id).unwrap();
        assert!(plan.thread_groups[0].requests[0].assertions.is_empty());
    }

    #[test]
    fn remove_assertion_error_for_missing_assertion() {
        let (mut mgr, plan_id, group_id, req_id) =
            manager_with_request("Plan", "G", "R");
        let unknown = Uuid::new_v4();
        let result = mgr.remove_assertion(&plan_id, &group_id, &req_id, &unknown);
        assert!(result.is_err());
    }

    #[test]
    fn update_assertion_applies_partial_fields() {
        let (mut mgr, plan_id, group_id, req_id) =
            manager_with_request("Plan", "G", "R");
        let rule = serde_json::json!({"type": "status_code_equals", "expected": 200});
        let assertion = mgr
            .add_assertion(&plan_id, &group_id, &req_id, "Old Name".to_string(), rule)
            .unwrap();

        let new_rule = serde_json::json!({"type": "status_code_equals", "expected": 201});
        let updated = mgr
            .update_assertion(&plan_id, &group_id, &req_id, &assertion.id, Some("New Name".to_string()), Some(new_rule.clone()))
            .unwrap();
        assert_eq!(updated.name, "New Name");
        assert_eq!(updated.rule, new_rule);
    }

    #[test]
    fn update_assertion_error_for_missing_assertion() {
        let (mut mgr, plan_id, group_id, req_id) =
            manager_with_request("Plan", "G", "R");
        let unknown = Uuid::new_v4();
        let result = mgr.update_assertion(&plan_id, &group_id, &req_id, &unknown, None, None);
        assert!(result.is_err());
    }
}
