use std::collections::HashMap;

use uuid::Uuid;

use crate::plan::model::{HttpMethod, HttpRequest, LoopCount, TestPlan, ThreadGroup};

// ---------------------------------------------------------------------------
// Template constructors
// ---------------------------------------------------------------------------

/// A minimal REST API test with one thread group (1 virtual user) and three
/// sample requests: GET list, POST create, PUT update.
pub fn rest_api_test() -> TestPlan {
    let mut plan = TestPlan::new("REST API Test");
    plan.description =
        "A simple REST API test with GET, POST, and PUT sample requests.".to_string();

    let requests = vec![
        build_request(
            "GET Users",
            HttpMethod::Get,
            "https://api.example.com/users",
        ),
        build_request(
            "POST Create User",
            HttpMethod::Post,
            "https://api.example.com/users",
        ),
        build_request(
            "PUT Update User",
            HttpMethod::Put,
            "https://api.example.com/users/1",
        ),
    ];

    let tg = ThreadGroup {
        id: Uuid::new_v4(),
        name: "Main Thread Group".to_string(),
        num_threads: 1,
        ramp_up_seconds: 0,
        loop_count: LoopCount::Finite { count: 1 },
        requests,
        enabled: true,
    };

    plan.thread_groups.push(tg);
    plan
}

/// A load test with 10 concurrent virtual users, a 10-second ramp-up, and a
/// 60-second duration against a single endpoint.
pub fn load_test() -> TestPlan {
    let mut plan = TestPlan::new("Load Test");
    plan.description =
        "A load test with 10 virtual users ramping up over 10 seconds for 60 seconds.".to_string();

    let request = build_request(
        "GET Health Check",
        HttpMethod::Get,
        "https://api.example.com/health",
    );

    let tg = ThreadGroup {
        id: Uuid::new_v4(),
        name: "Load Thread Group".to_string(),
        num_threads: 10,
        ramp_up_seconds: 10,
        loop_count: LoopCount::Duration { seconds: 60 },
        requests: vec![request],
        enabled: true,
    };

    plan.thread_groups.push(tg);
    plan
}

/// A stress test with 100 concurrent virtual users, a 30-second ramp-up, and
/// a 120-second duration against a single endpoint.
pub fn stress_test() -> TestPlan {
    let mut plan = TestPlan::new("Stress Test");
    plan.description =
        "A stress test with 100 virtual users ramping up over 30 seconds for 120 seconds."
            .to_string();

    let request = build_request(
        "GET Health Check",
        HttpMethod::Get,
        "https://api.example.com/health",
    );

    let tg = ThreadGroup {
        id: Uuid::new_v4(),
        name: "Stress Thread Group".to_string(),
        num_threads: 100,
        ramp_up_seconds: 30,
        loop_count: LoopCount::Duration { seconds: 120 },
        requests: vec![request],
        enabled: true,
    };

    plan.thread_groups.push(tg);
    plan
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn build_request(name: &str, method: HttpMethod, url: &str) -> HttpRequest {
    HttpRequest {
        id: Uuid::new_v4(),
        name: name.to_string(),
        method,
        url: url.to_string(),
        headers: HashMap::new(),
        body: None,
        assertions: Vec::new(),
        extractors: Vec::new(),
        enabled: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::model::LoopCount;

    // -----------------------------------------------------------------------
    // rest_api_test
    // -----------------------------------------------------------------------

    #[test]
    fn rest_api_test_has_correct_name() {
        let plan = rest_api_test();
        assert_eq!(plan.name, "REST API Test");
    }

    #[test]
    fn rest_api_test_has_description() {
        let plan = rest_api_test();
        assert!(!plan.description.is_empty());
    }

    #[test]
    fn rest_api_test_has_one_thread_group() {
        let plan = rest_api_test();
        assert_eq!(plan.thread_groups.len(), 1);
    }

    #[test]
    fn rest_api_test_thread_group_has_three_requests() {
        let plan = rest_api_test();
        let tg = &plan.thread_groups[0];
        assert_eq!(tg.requests.len(), 3);
    }

    #[test]
    fn rest_api_test_has_get_post_put_methods() {
        let plan = rest_api_test();
        let methods: Vec<_> = plan.thread_groups[0]
            .requests
            .iter()
            .map(|r| r.method.clone())
            .collect();
        assert_eq!(methods, vec![HttpMethod::Get, HttpMethod::Post, HttpMethod::Put]);
    }

    #[test]
    fn rest_api_test_has_single_thread() {
        let plan = rest_api_test();
        assert_eq!(plan.thread_groups[0].num_threads, 1);
    }

    #[test]
    fn rest_api_test_loop_count_is_finite_one() {
        let plan = rest_api_test();
        assert!(matches!(
            plan.thread_groups[0].loop_count,
            LoopCount::Finite { count: 1 }
        ));
    }

    // -----------------------------------------------------------------------
    // load_test
    // -----------------------------------------------------------------------

    #[test]
    fn load_test_has_correct_name() {
        let plan = load_test();
        assert_eq!(plan.name, "Load Test");
    }

    #[test]
    fn load_test_has_10_threads() {
        let plan = load_test();
        assert_eq!(plan.thread_groups[0].num_threads, 10);
    }

    #[test]
    fn load_test_has_10s_ramp_up() {
        let plan = load_test();
        assert_eq!(plan.thread_groups[0].ramp_up_seconds, 10);
    }

    #[test]
    fn load_test_runs_for_60_seconds() {
        let plan = load_test();
        assert!(matches!(
            plan.thread_groups[0].loop_count,
            LoopCount::Duration { seconds: 60 }
        ));
    }

    #[test]
    fn load_test_has_one_request() {
        let plan = load_test();
        assert_eq!(plan.thread_groups[0].requests.len(), 1);
    }

    // -----------------------------------------------------------------------
    // stress_test
    // -----------------------------------------------------------------------

    #[test]
    fn stress_test_has_correct_name() {
        let plan = stress_test();
        assert_eq!(plan.name, "Stress Test");
    }

    #[test]
    fn stress_test_has_100_threads() {
        let plan = stress_test();
        assert_eq!(plan.thread_groups[0].num_threads, 100);
    }

    #[test]
    fn stress_test_has_30s_ramp_up() {
        let plan = stress_test();
        assert_eq!(plan.thread_groups[0].ramp_up_seconds, 30);
    }

    #[test]
    fn stress_test_runs_for_120_seconds() {
        let plan = stress_test();
        assert!(matches!(
            plan.thread_groups[0].loop_count,
            LoopCount::Duration { seconds: 120 }
        ));
    }

    // -----------------------------------------------------------------------
    // Common properties across all templates
    // -----------------------------------------------------------------------

    #[test]
    fn all_templates_have_format_version_1() {
        assert_eq!(rest_api_test().format_version, 1);
        assert_eq!(load_test().format_version, 1);
        assert_eq!(stress_test().format_version, 1);
    }

    #[test]
    fn all_templates_have_enabled_thread_groups() {
        assert!(rest_api_test().thread_groups[0].enabled);
        assert!(load_test().thread_groups[0].enabled);
        assert!(stress_test().thread_groups[0].enabled);
    }

    #[test]
    fn all_templates_have_enabled_requests() {
        for plan in [rest_api_test(), load_test(), stress_test()] {
            for tg in &plan.thread_groups {
                for req in &tg.requests {
                    assert!(req.enabled, "Request '{}' should be enabled", req.name);
                }
            }
        }
    }

    #[test]
    fn all_templates_have_unique_ids() {
        let plans = [rest_api_test(), load_test(), stress_test()];
        let mut ids: Vec<_> = plans.iter().map(|p| p.id).collect();
        let unique_count = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), unique_count);
    }

    #[test]
    fn all_templates_requests_have_valid_urls() {
        for plan in [rest_api_test(), load_test(), stress_test()] {
            for tg in &plan.thread_groups {
                for req in &tg.requests {
                    assert!(
                        req.url.starts_with("https://"),
                        "URL '{}' should start with https://",
                        req.url
                    );
                }
            }
        }
    }
}
