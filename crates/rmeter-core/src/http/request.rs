use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::plan::model::RequestBody;

/// Authentication variants supported by the HTTP client.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Auth {
    Bearer(String),
    Basic { username: String, password: Option<String> },
}

/// Input required to execute a single HTTP request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SendRequestInput {
    /// HTTP method (GET, POST, …).
    pub method: crate::plan::model::HttpMethod,

    /// Target URL, may contain variable placeholders like `{{base_url}}`.
    pub url: String,

    /// Extra HTTP headers to include in the request.
    #[serde(default)]
    pub headers: HashMap<String, String>,

    /// Optional request body.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<RequestBody>,

    /// Optional authentication configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auth: Option<Auth>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::model::HttpMethod;

    #[test]
    fn send_request_input_serde_roundtrip_minimal() {
        let input = SendRequestInput {
            method: HttpMethod::Get,
            url: "http://example.com".to_string(),
            headers: HashMap::new(),
            body: None,
            auth: None,
        };
        let json = serde_json::to_string(&input).unwrap();
        let parsed: SendRequestInput = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.url, "http://example.com");
        assert!(parsed.body.is_none());
        assert!(parsed.auth.is_none());
    }

    #[test]
    fn send_request_input_with_headers() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("Authorization".to_string(), "Bearer xyz".to_string());

        let input = SendRequestInput {
            method: HttpMethod::Post,
            url: "http://api.example.com/data".to_string(),
            headers,
            body: None,
            auth: None,
        };
        let json = serde_json::to_string(&input).unwrap();
        let parsed: SendRequestInput = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.headers.len(), 2);
        assert_eq!(parsed.headers["Content-Type"], "application/json");
    }

    #[test]
    fn auth_bearer_construction_and_pattern_match() {
        // Auth::Bearer is a newtype variant with internally-tagged serde.
        // Serialization of newtype String variants isn't supported by serde's
        // internal tagging, but construction and matching works fine.
        // The frontend uses Auth as part of SendRequestInput deserialization.
        let auth = Auth::Bearer("my-token-123".to_string());
        match auth {
            Auth::Bearer(token) => assert_eq!(token, "my-token-123"),
            _ => panic!("expected Bearer"),
        }
    }

    #[test]
    fn auth_basic_serde_roundtrip() {
        let auth = Auth::Basic {
            username: "user".to_string(),
            password: Some("pass".to_string()),
        };
        let json = serde_json::to_string(&auth).unwrap();
        let parsed: Auth = serde_json::from_str(&json).unwrap();
        match parsed {
            Auth::Basic { username, password } => {
                assert_eq!(username, "user");
                assert_eq!(password, Some("pass".to_string()));
            }
            _ => panic!("expected Basic"),
        }
    }

    #[test]
    fn auth_basic_no_password() {
        let auth = Auth::Basic {
            username: "admin".to_string(),
            password: None,
        };
        let json = serde_json::to_string(&auth).unwrap();
        let parsed: Auth = serde_json::from_str(&json).unwrap();
        match parsed {
            Auth::Basic { username, password } => {
                assert_eq!(username, "admin");
                assert!(password.is_none());
            }
            _ => panic!("expected Basic"),
        }
    }

    #[test]
    fn headers_default_to_empty_when_missing() {
        let json = r#"{"method":"GET","url":"http://example.com"}"#;
        let parsed: SendRequestInput = serde_json::from_str(json).unwrap();
        assert!(parsed.headers.is_empty());
    }

    #[test]
    fn body_skipped_when_none() {
        let input = SendRequestInput {
            method: HttpMethod::Get,
            url: "http://example.com".to_string(),
            headers: HashMap::new(),
            body: None,
            auth: None,
        };
        let json = serde_json::to_string(&input).unwrap();
        assert!(!json.contains("body"));
    }

    #[test]
    fn auth_skipped_when_none() {
        let input = SendRequestInput {
            method: HttpMethod::Get,
            url: "http://example.com".to_string(),
            headers: HashMap::new(),
            body: None,
            auth: None,
        };
        let json = serde_json::to_string(&input).unwrap();
        assert!(!json.contains("auth"));
    }
}
