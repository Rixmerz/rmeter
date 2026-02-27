use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// The result of executing a single HTTP request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SendRequestOutput {
    /// HTTP response status code (e.g. 200, 404).
    pub status: u16,

    /// Response headers as a flat key/value map. When the server returns
    /// multiple values for the same header, only the last value is kept.
    pub headers: HashMap<String, String>,

    /// Response body decoded as UTF-8 (replacement characters for invalid
    /// sequences).
    pub body: String,

    /// Total round-trip time in milliseconds, measured from just before
    /// `send()` to just after the body is fully received.
    pub elapsed_ms: u64,

    /// Number of bytes in the raw response body.
    pub size_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_request_output_serde_roundtrip() {
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());

        let output = SendRequestOutput {
            status: 200,
            headers,
            body: "{\"ok\": true}".to_string(),
            elapsed_ms: 42,
            size_bytes: 12,
        };
        let json = serde_json::to_string(&output).unwrap();
        let parsed: SendRequestOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.status, 200);
        assert_eq!(parsed.body, "{\"ok\": true}");
        assert_eq!(parsed.elapsed_ms, 42);
        assert_eq!(parsed.size_bytes, 12);
        assert_eq!(parsed.headers["content-type"], "application/json");
    }

    #[test]
    fn send_request_output_empty_body() {
        let output = SendRequestOutput {
            status: 204,
            headers: HashMap::new(),
            body: String::new(),
            elapsed_ms: 5,
            size_bytes: 0,
        };
        let json = serde_json::to_string(&output).unwrap();
        let parsed: SendRequestOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.status, 204);
        assert!(parsed.body.is_empty());
        assert_eq!(parsed.size_bytes, 0);
    }

    #[test]
    fn send_request_output_error_status() {
        let output = SendRequestOutput {
            status: 500,
            headers: HashMap::new(),
            body: "Internal Server Error".to_string(),
            elapsed_ms: 150,
            size_bytes: 21,
        };
        let json = serde_json::to_string(&output).unwrap();
        let parsed: SendRequestOutput = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.status, 500);
    }

    #[test]
    fn send_request_output_clone() {
        let output = SendRequestOutput {
            status: 200,
            headers: HashMap::new(),
            body: "test".to_string(),
            elapsed_ms: 10,
            size_bytes: 4,
        };
        let cloned = output.clone();
        assert_eq!(cloned.status, output.status);
        assert_eq!(cloned.body, output.body);
    }
}
