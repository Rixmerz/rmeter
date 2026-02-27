//! Extractor engine — evaluates response extractors during a test run and
//! stores captured values into the variable map.

use std::collections::HashMap;

use regex::Regex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// ExtractorRule
// ---------------------------------------------------------------------------

/// The kind of extraction operation to perform.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExtractorRule {
    /// Extract a value using a simple dot-notation JSON path expression.
    JsonPath { expression: String },
    /// Extract a value using a regular expression capture group.
    Regex { pattern: String, group: u32 },
    /// Extract a response header value by name.
    Header { name: String },
}

// ---------------------------------------------------------------------------
// ExtractionResult
// ---------------------------------------------------------------------------

/// Result of evaluating a single extractor against an HTTP response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ExtractionResult {
    pub extractor_id: Uuid,
    pub extractor_name: String,
    /// The variable name that was written (or attempted to write).
    pub variable_name: String,
    /// Whether the extraction succeeded and a value was captured.
    pub success: bool,
    /// The extracted string value, or `None` if extraction failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extracted_value: Option<String>,
    /// Human-readable status message.
    pub message: String,
}

// ---------------------------------------------------------------------------
// ExtractionContext
// ---------------------------------------------------------------------------

/// Context needed to evaluate extractors against an HTTP response.
pub struct ExtractionContext<'a> {
    pub status_code: u16,
    pub headers: &'a HashMap<String, String>,
    pub body: &'a str,
}

// ---------------------------------------------------------------------------
// evaluate_extractor
// ---------------------------------------------------------------------------

/// Evaluate a single extractor rule against the response context.
///
/// Returns `(success, extracted_value, message)` — never panics.
pub fn evaluate_extractor(
    rule: &ExtractorRule,
    ctx: &ExtractionContext,
) -> (bool, Option<String>, String) {
    match rule {
        ExtractorRule::JsonPath { expression } => {
            match serde_json::from_str::<serde_json::Value>(ctx.body) {
                Ok(json) => match navigate_json_path(&json, expression) {
                    Some(value) => {
                        let extracted = json_value_to_string(value);
                        (
                            true,
                            Some(extracted.clone()),
                            format!("JSON path \"{}\" extracted \"{}\"", expression, extracted),
                        )
                    }
                    None => (
                        false,
                        None,
                        format!("JSON path \"{}\" not found in response body", expression),
                    ),
                },
                Err(e) => (
                    false,
                    None,
                    format!("Failed to parse response body as JSON: {e}"),
                ),
            }
        }

        ExtractorRule::Regex { pattern, group } => match Regex::new(pattern) {
            Ok(re) => match re.captures(ctx.body) {
                Some(caps) => {
                    let group_idx = *group as usize;
                    match caps.get(group_idx) {
                        Some(m) => {
                            let extracted = m.as_str().to_string();
                            (
                                true,
                                Some(extracted.clone()),
                                format!(
                                    "Regex \"{}\" group {} extracted \"{}\"",
                                    pattern, group, extracted
                                ),
                            )
                        }
                        None => (
                            false,
                            None,
                            format!(
                                "Regex \"{}\" matched but group {} does not exist",
                                pattern, group
                            ),
                        ),
                    }
                }
                None => (
                    false,
                    None,
                    format!("Regex \"{}\" did not match the response body", pattern),
                ),
            },
            Err(e) => (
                false,
                None,
                format!("Invalid regex pattern \"{}\": {e}", pattern),
            ),
        },

        ExtractorRule::Header { name } => {
            // Header names are lowercased when stored in the response map.
            let key = name.to_lowercase();
            match ctx.headers.get(&key) {
                Some(value) => (
                    true,
                    Some(value.clone()),
                    format!("Header \"{}\" extracted \"{}\"", name, value),
                ),
                None => (
                    false,
                    None,
                    format!("Header \"{}\" not found in response", name),
                ),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// evaluate_all
// ---------------------------------------------------------------------------

/// Evaluate all extractors configured on a request and return individual results.
///
/// Each extractor rule is deserialized from the generic `serde_json::Value`
/// stored in [`crate::plan::model::Extractor`].  Rules that cannot be parsed
/// produce a failing result with a descriptive message rather than panicking.
pub fn evaluate_all(
    extractors: &[crate::plan::model::Extractor],
    ctx: &ExtractionContext,
) -> Vec<ExtractionResult> {
    extractors
        .iter()
        .map(|extractor| {
            match serde_json::from_value::<ExtractorRule>(extractor.expression.clone()) {
                Ok(rule) => {
                    let (success, extracted_value, message) = evaluate_extractor(&rule, ctx);
                    ExtractionResult {
                        extractor_id: extractor.id,
                        extractor_name: extractor.name.clone(),
                        variable_name: extractor.variable.clone(),
                        success,
                        extracted_value,
                        message,
                    }
                }
                Err(e) => ExtractionResult {
                    extractor_id: extractor.id,
                    extractor_name: extractor.name.clone(),
                    variable_name: extractor.variable.clone(),
                    success: false,
                    extracted_value: None,
                    message: format!("Invalid extractor rule: {e}"),
                },
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// substitute_variables
// ---------------------------------------------------------------------------

/// Replace all `${varName}` placeholders in `input` with values from `variables`.
///
/// If a referenced variable is not found in the map, the placeholder is left
/// as-is so callers can detect unresolved references if needed.
pub fn substitute_variables(input: &str, variables: &HashMap<String, String>) -> String {
    // Fast path: nothing to substitute.
    if !input.contains("${") {
        return input.to_string();
    }

    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' {
            if chars.peek() == Some(&'{') {
                // Consume '{'.
                chars.next();

                // Collect variable name until '}'.
                let mut var_name = String::new();
                let mut closed = false;
                for c in chars.by_ref() {
                    if c == '}' {
                        closed = true;
                        break;
                    }
                    var_name.push(c);
                }

                if closed {
                    // Replace with variable value if found; otherwise restore placeholder.
                    if let Some(value) = variables.get(&var_name) {
                        result.push_str(value);
                    } else {
                        result.push('$');
                        result.push('{');
                        result.push_str(&var_name);
                        result.push('}');
                    }
                } else {
                    // Unclosed brace — restore what we consumed.
                    result.push('$');
                    result.push('{');
                    result.push_str(&var_name);
                }
            } else {
                result.push(ch);
            }
        } else {
            result.push(ch);
        }
    }

    result
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
/// expressions.
pub(crate) fn navigate_json_path<'a>(
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

/// Convert a `serde_json::Value` to a plain string for storage as a variable.
///
/// Strings are returned without surrounding quotes; other types use their JSON
/// representation.
fn json_value_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        other => other.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_ctx<'a>(
        headers: &'a HashMap<String, String>,
        body: &'a str,
    ) -> ExtractionContext<'a> {
        ExtractionContext {
            status_code: 200,
            headers,
            body,
        }
    }

    // --- JSON path tests ---

    #[test]
    fn json_path_simple_key() {
        let headers = HashMap::new();
        let ctx = make_ctx(&headers, r#"{"token":"abc123"}"#);
        let rule = ExtractorRule::JsonPath {
            expression: "token".to_string(),
        };
        let (success, value, _) = evaluate_extractor(&rule, &ctx);
        assert!(success);
        assert_eq!(value.as_deref(), Some("abc123"));
    }

    #[test]
    fn json_path_nested_key() {
        let headers = HashMap::new();
        let ctx = make_ctx(&headers, r#"{"data":{"id":42,"name":"Alice"}}"#);
        let rule = ExtractorRule::JsonPath {
            expression: "data.name".to_string(),
        };
        let (success, value, _) = evaluate_extractor(&rule, &ctx);
        assert!(success);
        assert_eq!(value.as_deref(), Some("Alice"));
    }

    #[test]
    fn json_path_array_index() {
        let headers = HashMap::new();
        let ctx = make_ctx(&headers, r#"{"items":["first","second","third"]}"#);
        let rule = ExtractorRule::JsonPath {
            expression: "items[1]".to_string(),
        };
        let (success, value, _) = evaluate_extractor(&rule, &ctx);
        assert!(success);
        assert_eq!(value.as_deref(), Some("second"));
    }

    #[test]
    fn json_path_not_found() {
        let headers = HashMap::new();
        let ctx = make_ctx(&headers, r#"{"a":1}"#);
        let rule = ExtractorRule::JsonPath {
            expression: "b.c".to_string(),
        };
        let (success, value, msg) = evaluate_extractor(&rule, &ctx);
        assert!(!success);
        assert!(value.is_none());
        assert!(msg.contains("not found"));
    }

    // --- Regex tests ---

    #[test]
    fn regex_with_capture_group() {
        let headers = HashMap::new();
        let ctx = make_ctx(&headers, "Order ID: 12345 confirmed");
        let rule = ExtractorRule::Regex {
            pattern: r"Order ID: (\d+)".to_string(),
            group: 1,
        };
        let (success, value, _) = evaluate_extractor(&rule, &ctx);
        assert!(success);
        assert_eq!(value.as_deref(), Some("12345"));
    }

    #[test]
    fn regex_no_match() {
        let headers = HashMap::new();
        let ctx = make_ctx(&headers, "Hello world");
        let rule = ExtractorRule::Regex {
            pattern: r"Order ID: (\d+)".to_string(),
            group: 1,
        };
        let (success, value, msg) = evaluate_extractor(&rule, &ctx);
        assert!(!success);
        assert!(value.is_none());
        assert!(msg.contains("did not match"));
    }

    // --- Header tests ---

    #[test]
    fn header_found() {
        let mut headers = HashMap::new();
        headers.insert("x-request-id".to_string(), "req-999".to_string());
        let ctx = make_ctx(&headers, "");
        let rule = ExtractorRule::Header {
            name: "x-request-id".to_string(),
        };
        let (success, value, _) = evaluate_extractor(&rule, &ctx);
        assert!(success);
        assert_eq!(value.as_deref(), Some("req-999"));
    }

    #[test]
    fn header_missing() {
        let headers = HashMap::new();
        let ctx = make_ctx(&headers, "");
        let rule = ExtractorRule::Header {
            name: "x-auth-token".to_string(),
        };
        let (success, value, msg) = evaluate_extractor(&rule, &ctx);
        assert!(!success);
        assert!(value.is_none());
        assert!(msg.contains("not found"));
    }

    // --- Invalid rule ---

    #[test]
    fn invalid_rule_in_evaluate_all() {
        let headers = HashMap::new();
        let ctx = make_ctx(&headers, "{}");
        let extractor = crate::plan::model::Extractor {
            id: Uuid::new_v4(),
            name: "bad".to_string(),
            variable: "x".to_string(),
            expression: serde_json::json!({ "type": "nonexistent_type" }),
        };
        let results = evaluate_all(&[extractor], &ctx);
        assert_eq!(results.len(), 1);
        assert!(!results[0].success);
        assert!(results[0].message.contains("Invalid extractor rule"));
    }

    // --- Variable substitution tests ---

    #[test]
    fn substitute_simple_variable() {
        let mut vars = HashMap::new();
        vars.insert("host".to_string(), "example.com".to_string());
        let result = substitute_variables("https://${host}/api", &vars);
        assert_eq!(result, "https://example.com/api");
    }

    #[test]
    fn substitute_multiple_variables() {
        let mut vars = HashMap::new();
        vars.insert("base".to_string(), "https://api.example.com".to_string());
        vars.insert("version".to_string(), "v2".to_string());
        let result = substitute_variables("${base}/${version}/users", &vars);
        assert_eq!(result, "https://api.example.com/v2/users");
    }

    #[test]
    fn substitute_missing_variable_left_as_is() {
        let vars = HashMap::new();
        let result = substitute_variables("url/${missing}/path", &vars);
        assert_eq!(result, "url/${missing}/path");
    }

    #[test]
    fn substitute_no_placeholders() {
        let vars = HashMap::new();
        let result = substitute_variables("https://example.com/api", &vars);
        assert_eq!(result, "https://example.com/api");
    }
}
