//! GraphQL over HTTP convenience layer.
//!
//! GraphQL uses standard HTTP POST requests with a JSON body.  This module
//! provides types and a helper to translate a GraphQL operation into an
//! [`SendRequestInput`] that the existing [`HttpClient`] can execute directly
//! without any additional protocol handling.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::plan::model::{HttpMethod, RequestBody};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A GraphQL operation sent over HTTP POST.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GraphQLRequest {
    /// The GraphQL query or mutation document.
    pub query: String,
    /// Optional variable bindings (`{"key": value, …}`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<serde_json::Value>,
    /// Optional name of the operation to execute in a multi-operation document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operation_name: Option<String>,
}

// ---------------------------------------------------------------------------
// Request builder
// ---------------------------------------------------------------------------

/// Translate a [`GraphQLRequest`] into the components needed for a plain HTTP
/// POST request.
///
/// Returns `(url, merged_headers, body)` ready to pass to [`crate::http::request::SendRequestInput`].
///
/// The caller-supplied `headers` are merged with a `Content-Type:
/// application/json` header (the caller's value wins on conflict so the
/// caller can override it if necessary, e.g. to use `application/graphql`).
///
/// # Errors
///
/// Returns an error string when the [`GraphQLRequest`] cannot be serialized to
/// JSON (this should never occur in practice because the input types are all
/// standard).
pub fn build_graphql_http_request(
    endpoint_url: &str,
    gql: &GraphQLRequest,
    extra_headers: &HashMap<String, String>,
) -> Result<(String, HashMap<String, String>, RequestBody), String> {
    // Serialize the GraphQL payload.
    let body_value = serde_json::to_value(gql)
        .map_err(|e| format!("Failed to serialize GraphQL request: {e}"))?;
    let body_str = body_value.to_string();

    // Build merged headers; caller values win over defaults.
    let mut headers = HashMap::new();
    headers.insert("Content-Type".to_owned(), "application/json".to_owned());
    headers.insert("Accept".to_owned(), "application/json".to_owned());
    for (k, v) in extra_headers {
        headers.insert(k.clone(), v.clone());
    }

    Ok((endpoint_url.to_owned(), headers, RequestBody::Raw(body_str)))
}

/// Build a [`crate::http::request::SendRequestInput`] for a GraphQL operation.
///
/// This is a convenience wrapper around [`build_graphql_http_request`] that
/// returns a fully-formed input struct instead of raw components.
pub fn graphql_to_send_request_input(
    endpoint_url: &str,
    gql: &GraphQLRequest,
    extra_headers: &HashMap<String, String>,
) -> Result<crate::http::request::SendRequestInput, String> {
    let (url, headers, body) =
        build_graphql_http_request(endpoint_url, gql, extra_headers)?;

    Ok(crate::http::request::SendRequestInput {
        method: HttpMethod::Post,
        url,
        headers,
        body: Some(body),
        auth: None,
    })
}

// ---------------------------------------------------------------------------
// Standard introspection query
// ---------------------------------------------------------------------------

/// The standard GraphQL introspection query.
///
/// Returns enough schema information to render type lists, field signatures,
/// and documentation in a UI.
pub const INTROSPECTION_QUERY: &str = r#"
{
  __schema {
    queryType { name }
    mutationType { name }
    subscriptionType { name }
    types {
      name
      kind
      description
      fields(includeDeprecated: true) {
        name
        description
        isDeprecated
        deprecationReason
        type {
          name
          kind
          ofType {
            name
            kind
            ofType {
              name
              kind
            }
          }
        }
        args {
          name
          description
          type {
            name
            kind
            ofType { name kind }
          }
          defaultValue
        }
      }
      inputFields {
        name
        description
        type { name kind ofType { name kind } }
        defaultValue
      }
      enumValues(includeDeprecated: true) {
        name
        description
        isDeprecated
        deprecationReason
      }
    }
  }
}
"#;

/// Build a [`GraphQLRequest`] for schema introspection.
pub fn introspection_request() -> GraphQLRequest {
    GraphQLRequest {
        query: INTROSPECTION_QUERY.to_owned(),
        variables: None,
        operation_name: None,
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_simple_query() {
        let gql = GraphQLRequest {
            query: "{ users { id name } }".to_owned(),
            variables: None,
            operation_name: None,
        };
        let (url, headers, body) =
            build_graphql_http_request("https://api.example.com/graphql", &gql, &HashMap::new())
                .expect("build should succeed");

        assert_eq!(url, "https://api.example.com/graphql");
        assert_eq!(headers.get("Content-Type").map(String::as_str), Some("application/json"));
        assert_eq!(headers.get("Accept").map(String::as_str), Some("application/json"));

        if let RequestBody::Raw(json_str) = body {
            let v: serde_json::Value =
                serde_json::from_str(&json_str).expect("body must be valid JSON");
            assert_eq!(v["query"], "{ users { id name } }");
            assert!(v.get("variables").is_none() || v["variables"].is_null());
        } else {
            panic!("Expected RequestBody::Raw");
        }
    }

    #[test]
    fn build_query_with_variables() {
        let gql = GraphQLRequest {
            query: "query GetUser($id: ID!) { user(id: $id) { name } }".to_owned(),
            variables: Some(serde_json::json!({ "id": "42" })),
            operation_name: Some("GetUser".to_owned()),
        };
        let (_, _, body) =
            build_graphql_http_request("https://api.example.com/graphql", &gql, &HashMap::new())
                .expect("build should succeed");

        if let RequestBody::Raw(json_str) = body {
            let v: serde_json::Value =
                serde_json::from_str(&json_str).expect("body must be valid JSON");
            assert_eq!(v["variables"]["id"], "42");
            assert_eq!(v["operationName"], "GetUser");
        } else {
            panic!("Expected RequestBody::Raw");
        }
    }

    #[test]
    fn caller_header_overrides_default_content_type() {
        let gql = GraphQLRequest {
            query: "{ __typename }".to_owned(),
            variables: None,
            operation_name: None,
        };
        let mut extra = HashMap::new();
        extra.insert("Content-Type".to_owned(), "application/graphql".to_owned());
        extra.insert("X-Custom".to_owned(), "yes".to_owned());

        let (_, headers, _) =
            build_graphql_http_request("https://api.example.com/graphql", &gql, &extra)
                .expect("build should succeed");

        // Caller's Content-Type wins.
        assert_eq!(
            headers.get("Content-Type").map(String::as_str),
            Some("application/graphql")
        );
        assert_eq!(headers.get("X-Custom").map(String::as_str), Some("yes"));
    }

    #[test]
    fn introspection_query_is_non_empty() {
        let req = introspection_request();
        assert!(req.query.contains("__schema"));
    }

    #[test]
    fn graphql_to_send_request_input_roundtrip() {
        let gql = GraphQLRequest {
            query: "{ ping }".to_owned(),
            variables: None,
            operation_name: None,
        };
        let input =
            graphql_to_send_request_input("https://gql.example.com/api", &gql, &HashMap::new())
                .expect("conversion should succeed");

        assert_eq!(input.url, "https://gql.example.com/api");
        assert!(matches!(input.method, HttpMethod::Post));
        assert!(input.body.is_some());
    }
}
