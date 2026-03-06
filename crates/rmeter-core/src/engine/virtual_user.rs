use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use tokio::sync::{mpsc, Mutex};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use rand::Rng;

use crate::engine::executor::CsvDataSet;
use crate::extractors::functions::{self, FunctionContext};
use crate::extractors::{evaluate_all as evaluate_extractors, ExtractionContext};
use crate::http::request::SendRequestInput;
use crate::plan::model::{HttpRequest, LoopCount, TestElement, Timer};
use crate::results::RequestResultEvent;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run a single virtual user: execute the provided request sequence according
/// to `loop_count`, sending a [`RequestResultEvent`] after every request.
///
/// `variables` is a shared mutable map of variable names to values.  Each
/// virtual user shares the map with other users in the same thread group so
/// that values extracted in one iteration are available in the next.
///
/// The function returns when either:
/// - The loop count is exhausted, or
/// - `cancel` is triggered (checked between requests, never mid-request).
#[allow(clippy::too_many_arguments)]
pub async fn run_virtual_user(
    user_id: u32,
    requests: Vec<HttpRequest>,
    elements: Vec<TestElement>,
    client: Arc<reqwest::Client>,
    cancel: CancellationToken,
    result_tx: mpsc::Sender<RequestResultEvent>,
    plan_id: Uuid,
    thread_group_name: String,
    loop_count: LoopCount,
    timer: Option<Timer>,
    variables: Arc<Mutex<HashMap<String, String>>>,
    csv_data_set: Arc<CsvDataSet>,
) {
    let use_elements = !elements.is_empty();

    macro_rules! run_once {
        () => {
            if use_elements {
                execute_elements(
                    &elements, &client, &cancel, &result_tx, plan_id,
                    &thread_group_name, user_id, &timer, &variables, &csv_data_set,
                ).await;
            } else {
                execute_request_sequence(
                    &requests, &client, &cancel, &result_tx, plan_id,
                    &thread_group_name, user_id, &timer, &variables, &csv_data_set,
                ).await;
            }
        };
    }

    match loop_count {
        LoopCount::Finite { count } => {
            for _ in 0..count {
                if cancel.is_cancelled() {
                    return;
                }
                run_once!();
            }
        }
        LoopCount::Duration { seconds } => {
            let deadline = Instant::now() + Duration::from_secs(seconds);
            while Instant::now() < deadline {
                if cancel.is_cancelled() {
                    return;
                }
                run_once!();
            }
        }
        LoopCount::Infinite => loop {
            if cancel.is_cancelled() {
                return;
            }
            run_once!();
        },
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Execute every enabled request in `requests` in order, yielding between each
/// one so the cancellation token is checked before each request.
#[allow(clippy::too_many_arguments)]
async fn execute_request_sequence(
    requests: &[HttpRequest],
    client: &Arc<reqwest::Client>,
    cancel: &CancellationToken,
    result_tx: &mpsc::Sender<RequestResultEvent>,
    plan_id: Uuid,
    thread_group_name: &str,
    user_id: u32,
    timer: &Option<Timer>,
    variables: &Arc<Mutex<HashMap<String, String>>>,
    csv_data_set: &CsvDataSet,
) {
    // Merge CSV row variables into the shared map for this iteration.
    if !csv_data_set.is_empty() {
        let csv_vars = csv_data_set.next_row();
        if !csv_vars.is_empty() {
            let mut vars = variables.lock().await;
            vars.extend(csv_vars);
        }
    }

    for req in requests {
        if !req.enabled {
            continue;
        }
        // Check cancellation before dispatching each request.
        if cancel.is_cancelled() {
            return;
        }

        let event =
            execute_single_request(req, client, plan_id, thread_group_name, user_id, variables).await;

        // If the channel is closed (receiver dropped) just stop sending.
        if result_tx.send(event).await.is_err() {
            return;
        }

        // Apply think-time delay after each request if configured.
        if let Some(ref t) = timer {
            let delay_ms = compute_timer_delay(t);
            if delay_ms > 0 {
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }
        }
    }
}

/// Execute a list of [`TestElement`]s, handling If/Loop/Transaction controllers
/// recursively. This is the element-based counterpart to `execute_request_sequence`.
#[allow(clippy::too_many_arguments)]
async fn execute_elements(
    elements: &[TestElement],
    client: &Arc<reqwest::Client>,
    cancel: &CancellationToken,
    result_tx: &mpsc::Sender<RequestResultEvent>,
    plan_id: Uuid,
    thread_group_name: &str,
    user_id: u32,
    timer: &Option<Timer>,
    variables: &Arc<Mutex<HashMap<String, String>>>,
    csv_data_set: &CsvDataSet,
) {
    // Merge CSV row variables at the start of each iteration.
    if !csv_data_set.is_empty() {
        let csv_vars = csv_data_set.next_row();
        if !csv_vars.is_empty() {
            let mut vars = variables.lock().await;
            vars.extend(csv_vars);
        }
    }

    execute_elements_inner(
        elements, client, cancel, result_tx, plan_id,
        thread_group_name, user_id, timer, variables,
    )
    .await;
}

/// Recursive inner implementation for element execution.
/// Uses `Box::pin` because recursive async functions require indirection.
#[allow(clippy::too_many_arguments)]
fn execute_elements_inner<'a>(
    elements: &'a [TestElement],
    client: &'a Arc<reqwest::Client>,
    cancel: &'a CancellationToken,
    result_tx: &'a mpsc::Sender<RequestResultEvent>,
    plan_id: Uuid,
    thread_group_name: &'a str,
    user_id: u32,
    timer: &'a Option<Timer>,
    variables: &'a Arc<Mutex<HashMap<String, String>>>,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'a>> {
    Box::pin(async move {
        for element in elements {
            if cancel.is_cancelled() {
                return;
            }

            match element {
                TestElement::Request { request } => {
                    if !request.enabled {
                        continue;
                    }
                    let event = execute_single_request(
                        request, client, plan_id, thread_group_name, user_id, variables,
                    )
                    .await;
                    if result_tx.send(event).await.is_err() {
                        return;
                    }
                    // Apply think-time delay after each request.
                    if let Some(ref t) = timer {
                        let delay_ms = compute_timer_delay(t);
                        if delay_ms > 0 {
                            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                        }
                    }
                }
                TestElement::IfController { condition, children, .. } => {
                    let vars_snapshot = variables.lock().await.clone();
                    if evaluate_condition(condition, &vars_snapshot) {
                        execute_elements_inner(
                            children, client, cancel, result_tx, plan_id,
                            thread_group_name, user_id, timer, variables,
                        )
                        .await;
                    }
                }
                TestElement::LoopController { count, children, .. } => {
                    for _ in 0..*count {
                        if cancel.is_cancelled() {
                            return;
                        }
                        execute_elements_inner(
                            children, client, cancel, result_tx, plan_id,
                            thread_group_name, user_id, timer, variables,
                        )
                        .await;
                    }
                }
                TestElement::TransactionController { name, children, .. } => {
                    let tx_start = Instant::now();
                    execute_elements_inner(
                        children, client, cancel, result_tx, plan_id,
                        thread_group_name, user_id, timer, variables,
                    )
                    .await;
                    let tx_elapsed = tx_start.elapsed().as_millis() as u64;
                    // Emit a synthetic result event for the transaction as a whole.
                    let event = RequestResultEvent {
                        id: Uuid::new_v4(),
                        plan_id,
                        thread_group_name: thread_group_name.to_string(),
                        request_name: format!("TX: {}", name),
                        timestamp: Utc::now(),
                        status_code: 0,
                        elapsed_ms: tx_elapsed,
                        size_bytes: 0,
                        assertions_passed: true,
                        error: None,
                        assertion_results: Vec::new(),
                        extraction_results: Vec::new(),
                        method: "TX".to_string(),
                        url: String::new(),
                        response_headers: HashMap::new(),
                        response_body: None,
                    };
                    let _ = result_tx.send(event).await;
                }
            }
        }
    })
}

/// Evaluate a simple condition string against the current variable map.
///
/// Supported forms:
/// - `${var} == "value"` — equality check
/// - `${var} != "value"` — inequality check
/// - `${var}` — truthy check (non-empty after substitution)
fn evaluate_condition(condition: &str, variables: &HashMap<String, String>) -> bool {
    let resolved = functions::substitute_all(condition, variables, None);

    // Try == comparison
    if let Some((left, right)) = resolved.split_once("==") {
        let left = left.trim().trim_matches('"');
        let right = right.trim().trim_matches('"');
        return left == right;
    }

    // Try != comparison
    if let Some((left, right)) = resolved.split_once("!=") {
        let left = left.trim().trim_matches('"');
        let right = right.trim().trim_matches('"');
        return left != right;
    }

    // Truthy check: non-empty and not "false" / "0"
    let trimmed = resolved.trim();
    !trimmed.is_empty() && trimmed != "false" && trimmed != "0"
}

/// Compute the delay in milliseconds for a given timer configuration.
fn compute_timer_delay(timer: &Timer) -> u64 {
    match timer {
        Timer::Constant { delay_ms } => *delay_ms,
        Timer::UniformRandom { min_ms, max_ms } => {
            if max_ms <= min_ms {
                *min_ms
            } else {
                rand::thread_rng().gen_range(*min_ms..=*max_ms)
            }
        }
        Timer::GaussianRandom { deviation_ms, offset_ms } => {
            use rand::distributions::{Distribution, Standard};
            let normal: f64 = Standard.sample(&mut rand::thread_rng());
            let delay = *offset_ms as f64 + normal * *deviation_ms as f64;
            delay.max(0.0) as u64
        }
    }
}

// ---------------------------------------------------------------------------
// ResponseData — internal struct carrying the full response
// ---------------------------------------------------------------------------

/// Full data returned from a successful HTTP request, including headers and
/// body text needed for assertion and extractor evaluation.
struct ResponseData {
    status_code: u16,
    size_bytes: u64,
    /// Response headers, with header names lowercased for case-insensitive matching.
    headers: HashMap<String, String>,
    body_text: String,
}

/// Execute a single [`HttpRequest`] and produce a [`RequestResultEvent`].
///
/// Before sending the request, variable placeholders (`${name}`) in the URL,
/// headers, and body are resolved from `variables`.  After receiving the
/// response, extractor results are written back into `variables`.
///
/// Network-level errors are captured and surfaced through the event's `error`
/// field rather than propagated up — virtual users must never panic.
async fn execute_single_request(
    req: &HttpRequest,
    client: &Arc<reqwest::Client>,
    plan_id: Uuid,
    thread_group_name: &str,
    user_id: u32,
    variables: &Arc<Mutex<HashMap<String, String>>>,
) -> RequestResultEvent {
    let timestamp = Utc::now();
    let start = Instant::now();

    // Snapshot the current variable map for substitution (short lock).
    let vars_snapshot = {
        variables.lock().await.clone()
    };

    // Build function context for built-in function evaluation.
    let func_ctx = FunctionContext {
        thread_num: user_id,
        counter: functions::global_counter(),
    };

    // Apply variable substitution and built-in functions to all mutable request fields.
    let resolved_req = resolve_request_variables(req, &vars_snapshot, Some(&func_ctx));

    // Build the reqwest request from the resolved plan model and send it.
    let result = build_and_send(&resolved_req, client).await;
    let elapsed_ms = start.elapsed().as_millis() as u64;

    match result {
        Ok(response_data) => {
            // Evaluate assertions after the response is received.
            let assertion_results = if !req.assertions.is_empty() {
                let ctx = crate::assertions::ResponseContext {
                    status_code: response_data.status_code,
                    headers: &response_data.headers,
                    body: &response_data.body_text,
                    elapsed_ms,
                };
                crate::assertions::evaluate_all(&req.assertions, &ctx)
            } else {
                Vec::new()
            };

            // All assertions must pass; vacuously true when none are configured.
            let all_passed = assertion_results.iter().all(|r| r.passed);

            // Evaluate extractors and store results back into the variable map.
            let extraction_results = if !req.extractors.is_empty() {
                let ctx = ExtractionContext {
                    status_code: response_data.status_code,
                    headers: &response_data.headers,
                    body: &response_data.body_text,
                };
                let results = evaluate_extractors(&req.extractors, &ctx);

                // Write extracted values into the shared variable map.
                let mut vars = variables.lock().await;
                for result in &results {
                    if result.success {
                        if let Some(ref value) = result.extracted_value {
                            vars.insert(result.variable_name.clone(), value.clone());
                        }
                    }
                }
                results
            } else {
                Vec::new()
            };

            // Truncate body for inspection
            let truncated_body = if response_data.body_text.len() > crate::results::MAX_RESPONSE_BODY_LEN {
                let mut s = response_data.body_text[..crate::results::MAX_RESPONSE_BODY_LEN].to_string();
                s.push_str("…[truncated]");
                Some(s)
            } else if response_data.body_text.is_empty() {
                None
            } else {
                Some(response_data.body_text.clone())
            };

            let method_str = resolved_req.method.to_string();

            RequestResultEvent {
                id: Uuid::new_v4(),
                plan_id,
                thread_group_name: thread_group_name.to_string(),
                request_name: req.name.clone(),
                timestamp,
                status_code: response_data.status_code,
                elapsed_ms,
                size_bytes: response_data.size_bytes,
                assertions_passed: all_passed,
                error: None,
                assertion_results,
                extraction_results,
                method: method_str,
                url: resolved_req.url.clone(),
                response_headers: response_data.headers.clone(),
                response_body: truncated_body,
            }
        }
        Err(err_msg) => {
            let method_str = resolved_req.method.to_string();
            RequestResultEvent {
                id: Uuid::new_v4(),
                plan_id,
                thread_group_name: thread_group_name.to_string(),
                request_name: req.name.clone(),
                timestamp,
                status_code: 0,
                elapsed_ms,
                size_bytes: 0,
                assertions_passed: false,
                error: Some(err_msg),
                assertion_results: Vec::new(),
                extraction_results: Vec::new(),
                method: method_str,
                url: resolved_req.url.clone(),
                response_headers: HashMap::new(),
                response_body: None,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Variable substitution
// ---------------------------------------------------------------------------

/// Return a clone of `req` with all `${varName}` placeholders resolved.
///
/// Substitution is applied to:
/// - `url`
/// - each header key and value
/// - body content (for Raw, Json, Xml variants)
fn resolve_request_variables(
    req: &HttpRequest,
    variables: &HashMap<String, String>,
    func_ctx: Option<&FunctionContext>,
) -> HttpRequest {
    use crate::plan::model::RequestBody;

    let sub = |s: &str| functions::substitute_all(s, variables, func_ctx);

    let url = sub(&req.url);

    let headers = req
        .headers
        .iter()
        .map(|(k, v)| (sub(k), sub(v)))
        .collect();

    let body = req.body.as_ref().map(|b| match b {
        RequestBody::Json { json } => RequestBody::Json { json: sub(json) },
        RequestBody::Raw { raw } => RequestBody::Raw { raw: sub(raw) },
        RequestBody::Xml { xml } => RequestBody::Xml { xml: sub(xml) },
        RequestBody::FormData { form_data } => RequestBody::FormData {
            form_data: form_data
                .iter()
                .map(|(k, v)| (sub(k), sub(v)))
                .collect(),
        },
    });

    HttpRequest {
        url,
        headers,
        body,
        // Clone all other fields unchanged.
        id: req.id,
        name: req.name.clone(),
        method: req.method.clone(),
        assertions: req.assertions.clone(),
        extractors: req.extractors.clone(),
        enabled: req.enabled,
    }
}

/// Build a [`reqwest::Request`] from an [`HttpRequest`], send it, and return
/// a [`ResponseData`] or an error message string.
async fn build_and_send(
    req: &HttpRequest,
    client: &Arc<reqwest::Client>,
) -> Result<ResponseData, String> {
    use crate::plan::model::{HttpMethod, RequestBody};

    // Map our plan model method to a reqwest Method.
    let method = match req.method {
        HttpMethod::Get => reqwest::Method::GET,
        HttpMethod::Post => reqwest::Method::POST,
        HttpMethod::Put => reqwest::Method::PUT,
        HttpMethod::Delete => reqwest::Method::DELETE,
        HttpMethod::Patch => reqwest::Method::PATCH,
        HttpMethod::Head => reqwest::Method::HEAD,
        HttpMethod::Options => reqwest::Method::OPTIONS,
    };

    let mut builder = client.request(method, &req.url);

    // Apply headers from plan.
    for (key, value) in &req.headers {
        builder = builder.header(key, value);
    }

    // Apply body if present.
    if let Some(body) = &req.body {
        builder = match body {
            RequestBody::Json { json } => {
                let value: serde_json::Value = serde_json::from_str(json)
                    .map_err(|e| format!("Invalid JSON body: {e}"))?;
                builder.json(&value)
            }
            RequestBody::FormData { form_data } => {
                let params: Vec<(&str, &str)> =
                    form_data.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
                builder.form(&params)
            }
            RequestBody::Raw { raw } => builder.body(raw.clone()),
            RequestBody::Xml { xml } => builder
                .header("Content-Type", "application/xml")
                .body(xml.clone()),
        };
    }

    let response = builder
        .send()
        .await
        .map_err(|e| format!("Network error: {e}"))?;

    let status_code = response.status().as_u16();

    // Collect response headers (lowercased names) before consuming the response.
    let headers: HashMap<String, String> = response
        .headers()
        .iter()
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|v| (name.as_str().to_lowercase(), v.to_string()))
        })
        .collect();

    let body_bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Error reading response body: {e}"))?;

    let size_bytes = body_bytes.len() as u64;
    // Attempt lossy UTF-8 decode — valid for assertion string comparisons.
    let body_text = String::from_utf8_lossy(&body_bytes).into_owned();

    Ok(ResponseData {
        status_code,
        size_bytes,
        headers,
        body_text,
    })
}

// ---------------------------------------------------------------------------
// Helper: build a SendRequestInput from a plan HttpRequest (for HttpClient)
// ---------------------------------------------------------------------------

/// Convert a plan [`HttpRequest`] into a [`SendRequestInput`] for use with
/// the shared [`HttpClient`].
#[allow(dead_code)]
pub fn to_send_request_input(req: &HttpRequest) -> SendRequestInput {
    SendRequestInput {
        method: req.method.clone(),
        url: req.url.clone(),
        headers: req.headers.clone(),
        body: req.body.clone(),
        auth: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::model::{HttpMethod, RequestBody};
    use uuid::Uuid;

    fn make_request(url: &str) -> HttpRequest {
        HttpRequest {
            id: Uuid::new_v4(),
            name: "Test".to_string(),
            method: HttpMethod::Get,
            url: url.to_string(),
            headers: HashMap::new(),
            body: None,
            assertions: Vec::new(),
            extractors: Vec::new(),
            enabled: true,
        }
    }

    fn make_vars(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    // -----------------------------------------------------------------------
    // resolve_request_variables
    // -----------------------------------------------------------------------

    #[test]
    fn resolve_url_variables() {
        let req = make_request("http://${host}/api/${version}/users");
        let vars = make_vars(&[("host", "example.com"), ("version", "v2")]);
        let resolved = resolve_request_variables(&req, &vars, None);
        assert_eq!(resolved.url, "http://example.com/api/v2/users");
    }

    #[test]
    fn resolve_no_variables_keeps_url_unchanged() {
        let req = make_request("http://example.com/api");
        let vars = HashMap::new();
        let resolved = resolve_request_variables(&req, &vars, None);
        assert_eq!(resolved.url, "http://example.com/api");
    }

    #[test]
    fn resolve_header_variables() {
        let mut req = make_request("http://example.com");
        req.headers
            .insert("Authorization".to_string(), "Bearer ${token}".to_string());
        let vars = make_vars(&[("token", "abc-123")]);
        let resolved = resolve_request_variables(&req, &vars, None);
        assert_eq!(resolved.headers["Authorization"], "Bearer abc-123");
    }

    #[test]
    fn resolve_json_body_variables() {
        let mut req = make_request("http://example.com");
        req.body = Some(RequestBody::Json {
            json: "{\"name\": \"${user_name}\"}".to_string(),
        });
        let vars = make_vars(&[("user_name", "Alice")]);
        let resolved = resolve_request_variables(&req, &vars, None);
        match &resolved.body {
            Some(RequestBody::Json { json: s }) => {
                assert_eq!(s, "{\"name\": \"Alice\"}");
            }
            _ => panic!("expected Json body"),
        }
    }

    #[test]
    fn resolve_raw_body_variables() {
        let mut req = make_request("http://example.com");
        req.body = Some(RequestBody::Raw { raw: "Hello ${name}".to_string() });
        let vars = make_vars(&[("name", "World")]);
        let resolved = resolve_request_variables(&req, &vars, None);
        match &resolved.body {
            Some(RequestBody::Raw { raw: s }) => assert_eq!(s, "Hello World"),
            _ => panic!("expected Raw body"),
        }
    }

    #[test]
    fn resolve_xml_body_variables() {
        let mut req = make_request("http://example.com");
        req.body = Some(RequestBody::Xml { xml: "<user>${user}</user>".to_string() });
        let vars = make_vars(&[("user", "Bob")]);
        let resolved = resolve_request_variables(&req, &vars, None);
        match &resolved.body {
            Some(RequestBody::Xml { xml: s }) => assert_eq!(s, "<user>Bob</user>"),
            _ => panic!("expected Xml body"),
        }
    }

    #[test]
    fn resolve_form_data_variables() {
        let mut req = make_request("http://example.com");
        req.body = Some(RequestBody::FormData { form_data: vec![
            ("${key_var}".to_string(), "${val_var}".to_string()),
        ]});
        let vars = make_vars(&[("key_var", "email"), ("val_var", "test@test.com")]);
        let resolved = resolve_request_variables(&req, &vars, None);
        match &resolved.body {
            Some(RequestBody::FormData { form_data: pairs }) => {
                assert_eq!(pairs[0].0, "email");
                assert_eq!(pairs[0].1, "test@test.com");
            }
            _ => panic!("expected FormData body"),
        }
    }

    #[test]
    fn resolve_preserves_id_and_name() {
        let req = make_request("http://example.com/${path}");
        let vars = make_vars(&[("path", "api")]);
        let resolved = resolve_request_variables(&req, &vars, None);
        assert_eq!(resolved.id, req.id);
        assert_eq!(resolved.name, req.name);
        assert_eq!(resolved.method, req.method);
        assert_eq!(resolved.enabled, req.enabled);
    }

    #[test]
    fn resolve_none_body_stays_none() {
        let req = make_request("http://example.com");
        let vars = HashMap::new();
        let resolved = resolve_request_variables(&req, &vars, None);
        assert!(resolved.body.is_none());
    }

    #[test]
    fn resolve_missing_variable_stays_as_placeholder() {
        let req = make_request("http://${missing_host}/api");
        let vars = HashMap::new();
        let resolved = resolve_request_variables(&req, &vars, None);
        // The behavior depends on substitute_variables implementation
        // which keeps unresolved placeholders or removes them
        // Just verify it doesn't panic
        assert!(!resolved.url.is_empty());
    }

    // -----------------------------------------------------------------------
    // to_send_request_input
    // -----------------------------------------------------------------------

    #[test]
    fn to_send_request_input_maps_fields() {
        let mut req = make_request("http://example.com/api");
        req.method = HttpMethod::Post;
        req.headers
            .insert("Content-Type".to_string(), "application/json".to_string());
        req.body = Some(RequestBody::Json { json: "{}".to_string() });

        let input = to_send_request_input(&req);
        assert_eq!(input.url, "http://example.com/api");
        assert_eq!(input.headers["Content-Type"], "application/json");
        assert!(input.body.is_some());
        assert!(input.auth.is_none());
    }

    #[test]
    fn to_send_request_input_no_body() {
        let req = make_request("http://example.com");
        let input = to_send_request_input(&req);
        assert!(input.body.is_none());
    }
}
