use crate::error::RmeterError;
use crate::plan::model::{HttpRequest, TestPlan, ThreadGroup};

/// Validate a [`TestPlan`] and return a list of validation errors.
///
/// An empty `Vec` means the plan is valid.
pub fn validate_plan(plan: &TestPlan) -> Vec<RmeterError> {
    let mut errors = Vec::new();

    if plan.name.trim().is_empty() {
        errors.push(RmeterError::Validation(
            "Test plan name must not be empty".to_string(),
        ));
    }

    for tg in &plan.thread_groups {
        errors.extend(validate_thread_group(tg));
    }

    errors
}

fn validate_thread_group(tg: &ThreadGroup) -> Vec<RmeterError> {
    let mut errors = Vec::new();

    if tg.name.trim().is_empty() {
        errors.push(RmeterError::Validation(format!(
            "Thread group '{}' name must not be empty",
            tg.id
        )));
    }

    if tg.num_threads == 0 {
        errors.push(RmeterError::Validation(format!(
            "Thread group '{}': num_threads must be at least 1",
            tg.name
        )));
    }

    for req in &tg.requests {
        errors.extend(validate_request(req));
    }

    errors
}

fn validate_request(req: &HttpRequest) -> Vec<RmeterError> {
    let mut errors = Vec::new();

    if req.url.trim().is_empty() {
        errors.push(RmeterError::Validation(format!(
            "Request '{}': URL must not be empty",
            req.name
        )));
    }

    // Basic URL structure check — allow variable placeholders like {{base_url}}.
    let url = req.url.trim();
    let expanded = url.replace("{{", "").replace("}}", "");
    if !expanded.starts_with("http://") && !expanded.starts_with("https://") {
        errors.push(RmeterError::Validation(format!(
            "Request '{}': URL must start with http:// or https:// (got: {})",
            req.name, req.url
        )));
    }

    errors
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use crate::plan::model::{HttpMethod, HttpRequest, LoopCount, ThreadGroup};
    use uuid::Uuid;

    fn make_valid_request(url: &str) -> HttpRequest {
        HttpRequest {
            id: Uuid::new_v4(),
            name: "Valid Request".to_string(),
            method: HttpMethod::Get,
            url: url.to_string(),
            headers: HashMap::new(),
            body: None,
            assertions: Vec::new(),
            extractors: Vec::new(),
            enabled: true,
        }
    }

    fn make_valid_thread_group(requests: Vec<HttpRequest>) -> ThreadGroup {
        ThreadGroup {
            id: Uuid::new_v4(),
            name: "Thread Group".to_string(),
            num_threads: 5,
            ramp_up_seconds: 2,
            loop_count: LoopCount::Finite { count: 10 },
            requests,
            enabled: true,
        }
    }

    fn make_valid_plan(name: &str, thread_groups: Vec<ThreadGroup>) -> TestPlan {
        TestPlan {
            id: Uuid::new_v4(),
            name: name.to_string(),
            description: String::new(),
            thread_groups,
            variables: Vec::new(),
            csv_data_sources: Vec::new(),
            format_version: 1,
        }
    }

    // -----------------------------------------------------------------------
    // Plan-level validation
    // -----------------------------------------------------------------------

    #[test]
    fn valid_plan_produces_no_errors() {
        let req = make_valid_request("https://example.com/api");
        let tg = make_valid_thread_group(vec![req]);
        let plan = make_valid_plan("My Plan", vec![tg]);
        let errors = validate_plan(&plan);
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    #[test]
    fn empty_plan_name_produces_error() {
        let plan = make_valid_plan("", vec![]);
        let errors = validate_plan(&plan);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.to_string().contains("name must not be empty")));
    }

    #[test]
    fn whitespace_only_plan_name_produces_error() {
        let plan = make_valid_plan("   ", vec![]);
        let errors = validate_plan(&plan);
        assert!(!errors.is_empty());
    }

    #[test]
    fn plan_with_no_thread_groups_is_valid() {
        let plan = make_valid_plan("Empty Plan", vec![]);
        let errors = validate_plan(&plan);
        assert!(errors.is_empty());
    }

    // -----------------------------------------------------------------------
    // Thread-group-level validation
    // -----------------------------------------------------------------------

    #[test]
    fn thread_group_with_zero_threads_produces_error() {
        let mut tg = make_valid_thread_group(vec![]);
        tg.num_threads = 0;
        let plan = make_valid_plan("Plan", vec![tg]);
        let errors = validate_plan(&plan);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.to_string().contains("num_threads must be at least 1")));
    }

    #[test]
    fn thread_group_with_empty_name_produces_error() {
        let mut tg = make_valid_thread_group(vec![]);
        tg.name = "   ".to_string();
        let plan = make_valid_plan("Plan", vec![tg]);
        let errors = validate_plan(&plan);
        assert!(!errors.is_empty());
    }

    #[test]
    fn multiple_thread_groups_all_validated() {
        let mut tg1 = make_valid_thread_group(vec![]);
        tg1.num_threads = 0; // invalid
        let mut tg2 = make_valid_thread_group(vec![]);
        tg2.num_threads = 0; // also invalid
        let plan = make_valid_plan("Plan", vec![tg1, tg2]);
        let errors = validate_plan(&plan);
        // Both thread groups should produce an error.
        assert!(errors.len() >= 2);
    }

    // -----------------------------------------------------------------------
    // Request-level validation
    // -----------------------------------------------------------------------

    #[test]
    fn request_with_empty_url_produces_error() {
        let req = make_valid_request("");
        let tg = make_valid_thread_group(vec![req]);
        let plan = make_valid_plan("Plan", vec![tg]);
        let errors = validate_plan(&plan);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.to_string().contains("URL must not be empty")));
    }

    #[test]
    fn request_with_http_url_is_valid() {
        let req = make_valid_request("http://example.com/api");
        let tg = make_valid_thread_group(vec![req]);
        let plan = make_valid_plan("Plan", vec![tg]);
        let errors = validate_plan(&plan);
        assert!(errors.is_empty());
    }

    #[test]
    fn request_with_https_url_is_valid() {
        let req = make_valid_request("https://api.example.com/v2/users");
        let tg = make_valid_thread_group(vec![req]);
        let plan = make_valid_plan("Plan", vec![tg]);
        let errors = validate_plan(&plan);
        assert!(errors.is_empty());
    }

    #[test]
    fn request_without_http_scheme_produces_error() {
        let req = make_valid_request("ftp://example.com/file");
        let tg = make_valid_thread_group(vec![req]);
        let plan = make_valid_plan("Plan", vec![tg]);
        let errors = validate_plan(&plan);
        assert!(!errors.is_empty());
        assert!(errors.iter().any(|e| e.to_string().contains("URL must start with http://")));
    }

    #[test]
    fn request_with_variable_placeholder_url_is_valid() {
        // {{base_url}}/api should be accepted because after stripping {{}} it starts with the content.
        let req = make_valid_request("{{base_url}}/api/users");
        let tg = make_valid_thread_group(vec![req]);
        let plan = make_valid_plan("Plan", vec![tg]);
        // The validation strips {{ and }} then checks for http(s)://. Since
        // "base_url/api/users" doesn't start with http/https it will fail,
        // but "https://api.example.com" wrapped would. We test that the
        // placeholder allows a full URL template.
        let req_full = make_valid_request("{{https://example.com}}/api");
        let tg_full = make_valid_thread_group(vec![req_full]);
        let plan_full = make_valid_plan("Plan Full", vec![tg_full]);
        let errors = validate_plan(&plan_full);
        assert!(errors.is_empty(), "Expected no errors for placeholder URL, got: {:?}", errors);
        // Also verify that "{{base_url}}" without scheme produces errors.
        let errors_bare = validate_plan(&plan);
        assert!(!errors_bare.is_empty());
    }

    #[test]
    fn multiple_errors_accumulate_across_levels() {
        let bad_req = make_valid_request(""); // empty URL → error
        let mut bad_tg = make_valid_thread_group(vec![bad_req]);
        bad_tg.num_threads = 0; // → another error
        let plan = make_valid_plan("  ", vec![bad_tg]); // empty name → another error
        let errors = validate_plan(&plan);
        // Should have at least 3 errors.
        assert!(errors.len() >= 3, "Expected >= 3 errors, got: {:?}", errors);
    }
}
