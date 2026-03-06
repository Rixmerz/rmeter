//! Extractor engine — evaluates response extractors during a test run and
//! stores captured values into the variable map.

pub mod functions;

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
/// Also evaluates JMeter-compatible functions:
/// - `${__UUID()}` — generates a random UUID v4
/// - `${__timeShift(format,,offset,,)}` — date/time with offset (e.g. `P30D`)
/// - `${__time(format)}` — current date/time in the given format
///
/// If a referenced variable is not found in the map and is not a recognized
/// function, the placeholder is left as-is so callers can detect unresolved
/// references if needed.
pub fn substitute_variables(input: &str, variables: &HashMap<String, String>) -> String {
    // Fast path: nothing to substitute.
    if !input.contains("${") {
        return input.to_string();
    }

    let mut result = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if i + 1 < len && bytes[i] == b'$' && bytes[i + 1] == b'{' {
            // Try to extract the full ${...} expression, handling nested braces.
            if let Some((expr, end)) = extract_placeholder(input, i + 2) {
                // First resolve any nested ${...} inside the expression.
                let resolved_expr = if expr.contains("${") {
                    substitute_variables(&expr, variables)
                } else {
                    expr.clone()
                };

                // Try variable lookup first, then JMeter function evaluation.
                if let Some(value) = variables.get(&resolved_expr) {
                    result.push_str(value);
                } else if let Some(value) = evaluate_jmeter_function(&resolved_expr) {
                    result.push_str(&value);
                } else {
                    // Unresolved — keep the placeholder.
                    result.push_str("${");
                    result.push_str(&resolved_expr);
                    result.push('}');
                }
                i = end;
            } else {
                // Unclosed brace — keep the '$' and continue.
                result.push('$');
                i += 1;
            }
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }

    result
}

/// Extract a `${...}` expression starting after the `${`, handling nested braces.
/// Returns `(content, end_index)` where `end_index` is the position after the closing `}`.
fn extract_placeholder(input: &str, start: usize) -> Option<(String, usize)> {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut depth = 1;
    let mut i = start;

    while i < len && depth > 0 {
        if i + 1 < len && bytes[i] == b'$' && bytes[i + 1] == b'{' {
            depth += 1;
            i += 2;
        } else if bytes[i] == b'}' {
            depth -= 1;
            if depth == 0 {
                let content = input[start..i].to_string();
                return Some((content, i + 1));
            }
            i += 1;
        } else {
            i += 1;
        }
    }

    None
}

/// Evaluate a JMeter-style function expression.
///
/// Supported functions:
/// - `__UUID()` — random UUID v4
/// - `__timeShift(format,,offset,,)` — current time shifted by an ISO-8601 duration
/// - `__time(format)` — current time in the given format
fn evaluate_jmeter_function(expr: &str) -> Option<String> {
    if expr == "__UUID()" {
        return Some(uuid::Uuid::new_v4().to_string());
    }

    if let Some(args_str) = strip_function_call(expr, "__timeShift") {
        let args: Vec<&str> = args_str.split(',').collect();
        let format = args.first().copied().unwrap_or("yyyy-MM-dd");
        let offset = args.get(2).copied().unwrap_or("");
        return Some(evaluate_time_shift(format, offset));
    }

    if let Some(args_str) = strip_function_call(expr, "__time") {
        let format = args_str.trim_matches(',').trim();
        let format = if format.is_empty() { "yyyy-MM-dd" } else { format };
        return Some(format_current_time(format));
    }

    None
}

/// Strip function name and parentheses from an expression like `__timeShift(args)`.
fn strip_function_call<'a>(expr: &'a str, func_name: &str) -> Option<&'a str> {
    let trimmed = expr.trim();
    if trimmed.starts_with(func_name) {
        let rest = &trimmed[func_name.len()..];
        if rest.starts_with('(') && rest.ends_with(')') {
            return Some(&rest[1..rest.len() - 1]);
        }
    }
    None
}

/// Evaluate `__timeShift(format,,offset,,)` — returns current time + offset.
///
/// Supports a subset of ISO-8601 durations: `P30D`, `P1M`, `P1Y`, `P7D`, etc.
/// Also supports negative offsets as JMeter variables that have already been
/// resolved (e.g. `P-30D`).
fn evaluate_time_shift(format: &str, offset: &str) -> String {
    use chrono::{Duration, Utc};

    let now = Utc::now();
    let shifted = if offset.is_empty() {
        now
    } else {
        parse_iso_duration_and_shift(now, offset)
    };

    format_chrono_time(format, shifted)
}

/// Format current time.
fn format_current_time(format: &str) -> String {
    format_chrono_time(format, chrono::Utc::now())
}

/// Parse a simple ISO-8601 duration and apply it to a datetime.
fn parse_iso_duration_and_shift(
    base: chrono::DateTime<chrono::Utc>,
    duration_str: &str,
) -> chrono::DateTime<chrono::Utc> {
    use chrono::{Duration, Months};

    let s = duration_str.trim();
    let s = if s.starts_with('P') || s.starts_with('p') {
        &s[1..]
    } else {
        // Not a valid duration, return base unchanged.
        return base;
    };

    // Simple parser for PnD, PnM, PnY patterns.
    let negative = s.starts_with('-');
    let s = if negative { &s[1..] } else { s };

    if s.ends_with('D') || s.ends_with('d') {
        if let Ok(days) = s[..s.len() - 1].parse::<i64>() {
            let days = if negative { -days } else { days };
            return base + Duration::days(days);
        }
    } else if s.ends_with('M') || s.ends_with('m') {
        if let Ok(months) = s[..s.len() - 1].parse::<u32>() {
            if negative {
                return base - Months::new(months);
            } else {
                return base + Months::new(months);
            }
        }
    } else if s.ends_with('Y') || s.ends_with('y') {
        if let Ok(years) = s[..s.len() - 1].parse::<u32>() {
            let months = years * 12;
            if negative {
                return base - Months::new(months);
            } else {
                return base + Months::new(months);
            }
        }
    }

    base
}

/// Convert a JMeter-style date format to chrono output.
///
/// Common JMeter patterns:
/// - `yyyy-MM-dd` → `2024-01-15`
/// - `yyyyMMdd` → `20240115`
/// - `yyyy-MM-dd'T'HH:mm:ss` → `2024-01-15T10:30:00`
fn format_chrono_time(jmeter_fmt: &str, dt: chrono::DateTime<chrono::Utc>) -> String {
    // Convert JMeter format tokens to chrono strftime tokens.
    let chrono_fmt = jmeter_fmt
        .replace("yyyy", "%Y")
        .replace("MM", "%m")
        .replace("dd", "%d")
        .replace("HH", "%H")
        .replace("mm", "%M")
        .replace("ss", "%S")
        .replace("SSS", "%3f")
        .replace('\'', "");

    dt.format(&chrono_fmt).to_string()
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

    // --- JMeter function tests ---

    #[test]
    fn substitute_uuid_function() {
        let vars = HashMap::new();
        let result = substitute_variables("id=${__UUID()}", &vars);
        assert!(result.starts_with("id="));
        let uuid_part = &result[3..];
        // UUID v4 is 36 chars with hyphens: 8-4-4-4-12
        assert_eq!(uuid_part.len(), 36);
        assert!(uuid_part.contains('-'));
    }

    #[test]
    fn substitute_time_function() {
        let vars = HashMap::new();
        let result = substitute_variables("date=${__time(yyyy-MM-dd)}", &vars);
        assert!(result.starts_with("date="));
        let date_part = &result[5..];
        // Should be YYYY-MM-DD format
        assert_eq!(date_part.len(), 10);
        assert_eq!(date_part.as_bytes()[4], b'-');
        assert_eq!(date_part.as_bytes()[7], b'-');
    }

    #[test]
    fn substitute_time_shift_function() {
        let vars = HashMap::new();
        let result = substitute_variables("date=${__timeShift(yyyy-MM-dd,,P30D,,)}", &vars);
        assert!(result.starts_with("date="));
        let date_part = &result[5..];
        assert_eq!(date_part.len(), 10);
        assert_eq!(date_part.as_bytes()[4], b'-');
    }

    #[test]
    fn substitute_nested_variable_in_function() {
        let mut vars = HashMap::new();
        vars.insert("first-date".to_string(), "P30D".to_string());
        let result = substitute_variables("${__timeShift(yyyy-MM-dd,,${first-date},,)}", &vars);
        // Should resolve the nested variable and evaluate timeShift
        assert_eq!(result.len(), 10); // YYYY-MM-DD
        assert_eq!(result.as_bytes()[4], b'-');
    }

    #[test]
    fn substitute_time_shift_yyyymmdd_format() {
        let vars = HashMap::new();
        let result = substitute_variables("${__timeShift(yyyyMMdd,,P1D,,)}", &vars);
        assert_eq!(result.len(), 8); // YYYYMMDD
    }
}
