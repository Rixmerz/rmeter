//! Assertion engine — evaluates HTTP response assertions during a test run.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// AssertionRule
// ---------------------------------------------------------------------------

/// The kind of assertion to evaluate against an HTTP response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AssertionRule {
    /// Assert that the HTTP status code equals a specific value.
    StatusCodeEquals { expected: u16 },
    /// Assert that the HTTP status code does NOT equal a specific value.
    StatusCodeNotEquals { not_expected: u16 },
    /// Assert that the HTTP status code falls within a range (inclusive).
    StatusCodeRange { min: u16, max: u16 },
    /// Assert that the response body contains a given substring.
    BodyContains { substring: String },
    /// Assert that the response body does NOT contain a given substring.
    BodyNotContains { substring: String },
    /// Assert that a simple dot-notation JSON path evaluates to a specific value.
    JsonPath { expression: String, expected: serde_json::Value },
    /// Assert that the response time is below a threshold in milliseconds.
    ResponseTimeBelow { threshold_ms: u64 },
    /// Assert that a response header equals a specific value.
    HeaderEquals { header: String, expected: String },
    /// Assert that a response header contains a specific substring.
    HeaderContains { header: String, substring: String },
}

// ---------------------------------------------------------------------------
// AssertionResult
// ---------------------------------------------------------------------------

/// Result of evaluating a single assertion against an HTTP response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AssertionResult {
    pub assertion_id: Uuid,
    pub assertion_name: String,
    pub passed: bool,
    pub message: String,
}

// ---------------------------------------------------------------------------
// ResponseContext
// ---------------------------------------------------------------------------

/// Context needed to evaluate assertions against an HTTP response.
pub struct ResponseContext<'a> {
    pub status_code: u16,
    pub headers: &'a std::collections::HashMap<String, String>,
    pub body: &'a str,
    pub elapsed_ms: u64,
}

// ---------------------------------------------------------------------------
// evaluate_assertion
// ---------------------------------------------------------------------------

/// Evaluate a single assertion rule against the response context.
///
/// Returns `(passed, message)` — never panics.
pub fn evaluate_assertion(rule: &AssertionRule, ctx: &ResponseContext) -> (bool, String) {
    match rule {
        AssertionRule::StatusCodeEquals { expected } => {
            let passed = ctx.status_code == *expected;
            let msg = if passed {
                format!("Status code {} matches expected {}", ctx.status_code, expected)
            } else {
                format!("Expected status {}, got {}", expected, ctx.status_code)
            };
            (passed, msg)
        }
        AssertionRule::StatusCodeNotEquals { not_expected } => {
            let passed = ctx.status_code != *not_expected;
            let msg = if passed {
                format!("Status code {} is not {}", ctx.status_code, not_expected)
            } else {
                format!("Status code {} should not be {}", ctx.status_code, not_expected)
            };
            (passed, msg)
        }
        AssertionRule::StatusCodeRange { min, max } => {
            let passed = ctx.status_code >= *min && ctx.status_code <= *max;
            let msg = if passed {
                format!("Status {} is within range [{}, {}]", ctx.status_code, min, max)
            } else {
                format!("Status {} is outside range [{}, {}]", ctx.status_code, min, max)
            };
            (passed, msg)
        }
        AssertionRule::BodyContains { substring } => {
            let passed = ctx.body.contains(substring.as_str());
            let msg = if passed {
                format!("Body contains \"{}\"", substring)
            } else {
                format!("Body does not contain \"{}\"", substring)
            };
            (passed, msg)
        }
        AssertionRule::BodyNotContains { substring } => {
            let passed = !ctx.body.contains(substring.as_str());
            let msg = if passed {
                format!("Body does not contain \"{}\"", substring)
            } else {
                format!("Body unexpectedly contains \"{}\"", substring)
            };
            (passed, msg)
        }
        AssertionRule::JsonPath { expression, expected } => {
            // Parse body as JSON, navigate the dot-notation path, compare with expected.
            match serde_json::from_str::<serde_json::Value>(ctx.body) {
                Ok(json) => {
                    let actual = navigate_json_path(&json, expression);
                    match actual {
                        Some(value) if value == expected => (
                            true,
                            format!("JSON path \"{}\" equals {:?}", expression, expected),
                        ),
                        Some(value) => (
                            false,
                            format!(
                                "JSON path \"{}\" expected {:?}, got {:?}",
                                expression, expected, value
                            ),
                        ),
                        None => (
                            false,
                            format!("JSON path \"{}\" not found in response", expression),
                        ),
                    }
                }
                Err(e) => (false, format!("Failed to parse response as JSON: {e}")),
            }
        }
        AssertionRule::ResponseTimeBelow { threshold_ms } => {
            let passed = ctx.elapsed_ms < *threshold_ms;
            let msg = if passed {
                format!(
                    "Response time {} ms < {} ms threshold",
                    ctx.elapsed_ms, threshold_ms
                )
            } else {
                format!(
                    "Response time {} ms exceeds {} ms threshold",
                    ctx.elapsed_ms, threshold_ms
                )
            };
            (passed, msg)
        }
        AssertionRule::HeaderEquals { header, expected } => {
            match ctx.headers.get(header) {
                Some(value) if value == expected => (
                    true,
                    format!("Header \"{}\" equals \"{}\"", header, expected),
                ),
                Some(value) => (
                    false,
                    format!(
                        "Header \"{}\" expected \"{}\", got \"{}\"",
                        header, expected, value
                    ),
                ),
                None => (
                    false,
                    format!("Header \"{}\" not found in response", header),
                ),
            }
        }
        AssertionRule::HeaderContains { header, substring } => {
            match ctx.headers.get(header) {
                Some(value) if value.contains(substring.as_str()) => (
                    true,
                    format!("Header \"{}\" contains \"{}\"", header, substring),
                ),
                Some(value) => (
                    false,
                    format!(
                        "Header \"{}\" value \"{}\" does not contain \"{}\"",
                        header, value, substring
                    ),
                ),
                None => (
                    false,
                    format!("Header \"{}\" not found in response", header),
                ),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// evaluate_all
// ---------------------------------------------------------------------------

/// Evaluate all assertions configured on a request and return individual results.
///
/// Each assertion rule is deserialized from the generic `serde_json::Value`
/// stored in [`crate::plan::model::Assertion`].  Rules that cannot be parsed
/// produce a failing result with a descriptive message rather than panicking.
pub fn evaluate_all(
    assertions: &[crate::plan::model::Assertion],
    ctx: &ResponseContext,
) -> Vec<AssertionResult> {
    assertions
        .iter()
        .map(|assertion| {
            match serde_json::from_value::<AssertionRule>(assertion.rule.clone()) {
                Ok(rule) => {
                    let (passed, message) = evaluate_assertion(&rule, ctx);
                    AssertionResult {
                        assertion_id: assertion.id,
                        assertion_name: assertion.name.clone(),
                        passed,
                        message,
                    }
                }
                Err(e) => AssertionResult {
                    assertion_id: assertion.id,
                    assertion_name: assertion.name.clone(),
                    passed: false,
                    message: format!("Invalid assertion rule: {e}"),
                },
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// JSON path navigator (simple dot-notation)
// ---------------------------------------------------------------------------

/// Navigate a simple dot-notation JSON path.
///
/// Supports:
/// - `"key"` — top-level key
/// - `"key.subkey"` — nested key
/// - `"key[0]"` — array index
/// - `"key[0].subkey"` — array index followed by key
///
/// Does NOT support bracket-notation key access, wildcards, or filter
/// expressions.  For advanced querying, a full JSONPath library would be
/// required, but this simple implementation avoids an extra dependency.
fn navigate_json_path<'a>(
    value: &'a serde_json::Value,
    path: &str,
) -> Option<&'a serde_json::Value> {
    let mut current = value;
    for segment in path.split('.') {
        // Check for array index notation: "items[0]"
        if let Some(bracket_pos) = segment.find('[') {
            let key = &segment[..bracket_pos];
            let closing = segment.rfind(']').unwrap_or(segment.len() - 1);
            let idx_str = &segment[bracket_pos + 1..closing];

            // Navigate into the object key (if a key precedes the bracket).
            if !key.is_empty() {
                current = current.get(key)?;
            }
            let idx: usize = idx_str.parse().ok()?;
            current = current.get(idx)?;
        } else {
            current = current.get(segment)?;
        }
    }
    Some(current)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_ctx<'a>(
        status: u16,
        headers: &'a HashMap<String, String>,
        body: &'a str,
        elapsed: u64,
    ) -> ResponseContext<'a> {
        ResponseContext {
            status_code: status,
            headers,
            body,
            elapsed_ms: elapsed,
        }
    }

    #[test]
    fn status_code_equals_pass() {
        let headers = HashMap::new();
        let ctx = make_ctx(200, &headers, "", 50);
        let rule = AssertionRule::StatusCodeEquals { expected: 200 };
        let (passed, _) = evaluate_assertion(&rule, &ctx);
        assert!(passed);
    }

    #[test]
    fn status_code_equals_fail() {
        let headers = HashMap::new();
        let ctx = make_ctx(404, &headers, "", 50);
        let rule = AssertionRule::StatusCodeEquals { expected: 200 };
        let (passed, msg) = evaluate_assertion(&rule, &ctx);
        assert!(!passed);
        assert!(msg.contains("404"));
    }

    #[test]
    fn status_code_not_equals_pass() {
        let headers = HashMap::new();
        let ctx = make_ctx(200, &headers, "", 50);
        let rule = AssertionRule::StatusCodeNotEquals { not_expected: 500 };
        let (passed, _) = evaluate_assertion(&rule, &ctx);
        assert!(passed);
    }

    #[test]
    fn status_code_range_pass() {
        let headers = HashMap::new();
        let ctx = make_ctx(201, &headers, "", 50);
        let rule = AssertionRule::StatusCodeRange { min: 200, max: 299 };
        let (passed, _) = evaluate_assertion(&rule, &ctx);
        assert!(passed);
    }

    #[test]
    fn status_code_range_fail() {
        let headers = HashMap::new();
        let ctx = make_ctx(404, &headers, "", 50);
        let rule = AssertionRule::StatusCodeRange { min: 200, max: 299 };
        let (passed, _) = evaluate_assertion(&rule, &ctx);
        assert!(!passed);
    }

    #[test]
    fn body_contains_pass() {
        let headers = HashMap::new();
        let ctx = make_ctx(200, &headers, "Hello, world!", 50);
        let rule = AssertionRule::BodyContains { substring: "world".to_string() };
        let (passed, _) = evaluate_assertion(&rule, &ctx);
        assert!(passed);
    }

    #[test]
    fn body_not_contains_pass() {
        let headers = HashMap::new();
        let ctx = make_ctx(200, &headers, "Hello, world!", 50);
        let rule = AssertionRule::BodyNotContains { substring: "error".to_string() };
        let (passed, _) = evaluate_assertion(&rule, &ctx);
        assert!(passed);
    }

    #[test]
    fn response_time_below_pass() {
        let headers = HashMap::new();
        let ctx = make_ctx(200, &headers, "", 99);
        let rule = AssertionRule::ResponseTimeBelow { threshold_ms: 100 };
        let (passed, _) = evaluate_assertion(&rule, &ctx);
        assert!(passed);
    }

    #[test]
    fn response_time_below_fail() {
        let headers = HashMap::new();
        let ctx = make_ctx(200, &headers, "", 200);
        let rule = AssertionRule::ResponseTimeBelow { threshold_ms: 100 };
        let (passed, _) = evaluate_assertion(&rule, &ctx);
        assert!(!passed);
    }

    #[test]
    fn header_equals_pass() {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());
        let ctx = make_ctx(200, &headers, "", 50);
        let rule = AssertionRule::HeaderEquals {
            header: "content-type".to_string(),
            expected: "application/json".to_string(),
        };
        let (passed, _) = evaluate_assertion(&rule, &ctx);
        assert!(passed);
    }

    #[test]
    fn header_contains_pass() {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json; charset=utf-8".to_string());
        let ctx = make_ctx(200, &headers, "", 50);
        let rule = AssertionRule::HeaderContains {
            header: "content-type".to_string(),
            substring: "application/json".to_string(),
        };
        let (passed, _) = evaluate_assertion(&rule, &ctx);
        assert!(passed);
    }

    #[test]
    fn header_missing_fails() {
        let headers = HashMap::new();
        let ctx = make_ctx(200, &headers, "", 50);
        let rule = AssertionRule::HeaderEquals {
            header: "x-custom".to_string(),
            expected: "value".to_string(),
        };
        let (passed, msg) = evaluate_assertion(&rule, &ctx);
        assert!(!passed);
        assert!(msg.contains("not found"));
    }

    #[test]
    fn json_path_simple_key() {
        let headers = HashMap::new();
        let ctx = make_ctx(200, &headers, r#"{"status":"ok"}"#, 50);
        let rule = AssertionRule::JsonPath {
            expression: "status".to_string(),
            expected: serde_json::json!("ok"),
        };
        let (passed, _) = evaluate_assertion(&rule, &ctx);
        assert!(passed);
    }

    #[test]
    fn json_path_nested() {
        let headers = HashMap::new();
        let ctx = make_ctx(200, &headers, r#"{"data":{"id":42}}"#, 50);
        let rule = AssertionRule::JsonPath {
            expression: "data.id".to_string(),
            expected: serde_json::json!(42),
        };
        let (passed, _) = evaluate_assertion(&rule, &ctx);
        assert!(passed);
    }

    #[test]
    fn json_path_array_index() {
        let headers = HashMap::new();
        let ctx = make_ctx(200, &headers, r#"{"items":["a","b","c"]}"#, 50);
        let rule = AssertionRule::JsonPath {
            expression: "items[1]".to_string(),
            expected: serde_json::json!("b"),
        };
        let (passed, _) = evaluate_assertion(&rule, &ctx);
        assert!(passed);
    }

    #[test]
    fn json_path_not_found() {
        let headers = HashMap::new();
        let ctx = make_ctx(200, &headers, r#"{"a":1}"#, 50);
        let rule = AssertionRule::JsonPath {
            expression: "b.c".to_string(),
            expected: serde_json::json!(1),
        };
        let (passed, msg) = evaluate_assertion(&rule, &ctx);
        assert!(!passed);
        assert!(msg.contains("not found"));
    }

    #[test]
    fn json_path_invalid_json_body() {
        let headers = HashMap::new();
        let ctx = make_ctx(200, &headers, "not json", 50);
        let rule = AssertionRule::JsonPath {
            expression: "key".to_string(),
            expected: serde_json::json!("val"),
        };
        let (passed, msg) = evaluate_assertion(&rule, &ctx);
        assert!(!passed);
        assert!(msg.contains("parse"));
    }
}
