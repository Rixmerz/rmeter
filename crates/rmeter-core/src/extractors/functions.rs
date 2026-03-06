//! Built-in functions for variable substitution.
//!
//! Functions follow the syntax `${__functionName(arg1,arg2)}` and are evaluated
//! during variable substitution in request URLs, headers, and bodies.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use rand::Rng;
use uuid::Uuid;

/// Thread-local context passed to function evaluation.
pub struct FunctionContext {
    /// The current virtual user / thread number.
    pub thread_num: u32,
    /// Shared counter for `__counter()`.
    pub counter: &'static AtomicU64,
}

/// A global atomic counter used by `__counter()`.
static GLOBAL_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Get a reference to the global counter (for passing into FunctionContext).
pub fn global_counter() -> &'static AtomicU64 {
    &GLOBAL_COUNTER
}

/// Substitute both `${varName}` variables AND `${__func(args)}` built-in
/// functions in the input string.
///
/// Variable substitution runs first, then function evaluation.
pub fn substitute_all(
    input: &str,
    variables: &HashMap<String, String>,
    ctx: Option<&FunctionContext>,
) -> String {
    // First pass: standard variable substitution.
    let after_vars = crate::extractors::substitute_variables(input, variables);

    // Second pass: function evaluation.
    if !after_vars.contains("${__") {
        return after_vars;
    }

    evaluate_functions(&after_vars, ctx)
}

/// Evaluate all `${__functionName(args)}` patterns in the input.
fn evaluate_functions(input: &str, ctx: Option<&FunctionContext>) -> String {
    let mut result = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '$' && chars.peek() == Some(&'{') {
            chars.next(); // consume '{'

            // Collect everything until '}'
            let mut expr = String::new();
            let mut closed = false;
            let mut depth = 1;
            for c in chars.by_ref() {
                if c == '{' {
                    depth += 1;
                } else if c == '}' {
                    depth -= 1;
                    if depth == 0 {
                        closed = true;
                        break;
                    }
                }
                expr.push(c);
            }

            if closed && expr.starts_with("__") {
                // Try to evaluate as a function.
                match evaluate_single_function(&expr, ctx) {
                    Some(value) => result.push_str(&value),
                    None => {
                        // Not a recognized function — restore placeholder.
                        result.push('$');
                        result.push('{');
                        result.push_str(&expr);
                        result.push('}');
                    }
                }
            } else if closed {
                // Not a function — restore as-is.
                result.push('$');
                result.push('{');
                result.push_str(&expr);
                result.push('}');
            } else {
                // Unclosed — restore what we consumed.
                result.push('$');
                result.push('{');
                result.push_str(&expr);
            }
        } else {
            result.push(ch);
        }
    }

    result
}

/// Parse and evaluate a single function expression like `__random(1,100)`.
///
/// Returns `Some(result)` if recognized, `None` otherwise.
fn evaluate_single_function(expr: &str, ctx: Option<&FunctionContext>) -> Option<String> {
    // Parse function name and arguments: "__funcName(arg1,arg2)" or "__funcName"
    let (func_name, args) = if let Some(paren_pos) = expr.find('(') {
        let name = &expr[..paren_pos];
        let args_str = expr[paren_pos + 1..].trim_end_matches(')');
        let args: Vec<&str> = if args_str.is_empty() {
            Vec::new()
        } else {
            args_str.split(',').map(|s| s.trim()).collect()
        };
        (name, args)
    } else {
        (expr.as_ref(), Vec::new())
    };

    match func_name {
        "__random" => fn_random(&args),
        "__randomString" => fn_random_string(&args),
        "__time" => fn_time(&args),
        "__uuid" | "__UUID" => Some(fn_uuid()),
        "__counter" => Some(fn_counter(ctx)),
        "__threadNum" => Some(fn_thread_num(ctx)),
        "__property" => fn_property(&args, ctx),
        _ => None,
    }
}

/// `__random(min,max)` — Random integer between min and max (inclusive).
fn fn_random(args: &[&str]) -> Option<String> {
    if args.len() < 2 {
        return Some("0".to_string());
    }
    let min: i64 = args[0].parse().unwrap_or(0);
    let max: i64 = args[1].parse().unwrap_or(100);
    if max <= min {
        return Some(min.to_string());
    }
    let value = rand::thread_rng().gen_range(min..=max);
    Some(value.to_string())
}

/// `__randomString(length)` — Random alphanumeric string of given length.
fn fn_random_string(args: &[&str]) -> Option<String> {
    let len: usize = args.first().and_then(|s| s.parse().ok()).unwrap_or(8);
    let s: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(len)
        .map(char::from)
        .collect();
    Some(s)
}

/// `__time()` or `__time(format)` — Current timestamp.
/// Without args: epoch milliseconds. With format: chrono strftime.
fn fn_time(args: &[&str]) -> Option<String> {
    let now = chrono::Utc::now();
    if args.is_empty() || args[0].is_empty() {
        Some(now.timestamp_millis().to_string())
    } else {
        Some(now.format(args[0]).to_string())
    }
}

/// `__uuid()` — Random UUID v4.
fn fn_uuid() -> String {
    Uuid::new_v4().to_string()
}

/// `__counter()` — Auto-incrementing counter (global).
fn fn_counter(ctx: Option<&FunctionContext>) -> String {
    let counter = ctx
        .map(|c| c.counter)
        .unwrap_or(&GLOBAL_COUNTER);
    counter.fetch_add(1, Ordering::Relaxed).to_string()
}

/// `__threadNum()` — Current thread/user number.
fn fn_thread_num(ctx: Option<&FunctionContext>) -> String {
    ctx.map(|c| c.thread_num.to_string())
        .unwrap_or_else(|| "0".to_string())
}

/// `__property(name,default)` — Not implemented for now, returns default or empty.
fn fn_property(args: &[&str], _ctx: Option<&FunctionContext>) -> Option<String> {
    if args.len() >= 2 {
        Some(args[1].to_string())
    } else {
        Some(String::new())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_returns_value_in_range() {
        let result = fn_random(&["1", "10"]).unwrap();
        let val: i64 = result.parse().unwrap();
        assert!((1..=10).contains(&val));
    }

    #[test]
    fn random_string_returns_correct_length() {
        let result = fn_random_string(&["16"]).unwrap();
        assert_eq!(result.len(), 16);
        assert!(result.chars().all(|c| c.is_alphanumeric()));
    }

    #[test]
    fn time_returns_epoch_millis() {
        let result = fn_time(&[]).unwrap();
        let val: i64 = result.parse().unwrap();
        assert!(val > 1_000_000_000_000); // after year ~2001
    }

    #[test]
    fn uuid_returns_valid_uuid() {
        let result = fn_uuid();
        assert!(Uuid::parse_str(&result).is_ok());
    }

    #[test]
    fn counter_increments() {
        let c = AtomicU64::new(1);
        let ctx = FunctionContext {
            thread_num: 0,
            counter: unsafe { std::mem::transmute::<&AtomicU64, &'static AtomicU64>(&c) },
        };
        let a = fn_counter(Some(&ctx));
        let b = fn_counter(Some(&ctx));
        let va: u64 = a.parse().unwrap();
        let vb: u64 = b.parse().unwrap();
        assert_eq!(vb, va + 1);
    }

    #[test]
    fn thread_num_returns_user_id() {
        let c = AtomicU64::new(1);
        let ctx = FunctionContext {
            thread_num: 42,
            counter: unsafe { std::mem::transmute::<&AtomicU64, &'static AtomicU64>(&c) },
        };
        assert_eq!(fn_thread_num(Some(&ctx)), "42");
    }

    #[test]
    fn evaluate_functions_replaces_patterns() {
        let input = "id=${__uuid()}";
        let result = evaluate_functions(input, None);
        assert!(result.starts_with("id="));
        assert!(result.len() > 10); // UUID is 36 chars
    }

    #[test]
    fn evaluate_functions_preserves_non_function_vars() {
        let input = "${normal_var} and ${__uuid()}";
        let result = evaluate_functions(input, None);
        assert!(result.starts_with("${normal_var} and "));
    }

    #[test]
    fn substitute_all_handles_both() {
        let mut vars = HashMap::new();
        vars.insert("host".to_string(), "example.com".to_string());
        let input = "https://${host}/api?id=${__random(1,100)}";
        let result = substitute_all(input, &vars, None);
        assert!(result.starts_with("https://example.com/api?id="));
    }
}
