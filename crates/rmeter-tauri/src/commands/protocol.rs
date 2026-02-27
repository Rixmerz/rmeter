//! Tauri commands for extended protocol support: WebSocket and GraphQL.

use std::collections::HashMap;

use rmeter_core::error::RmeterError;
use rmeter_core::http::client::HttpClient;
use rmeter_core::http::graphql::{introspection_request, GraphQLRequest, graphql_to_send_request_input};
use rmeter_core::http::response::SendRequestOutput;
use rmeter_core::http::websocket::{execute_websocket_scenario, WebSocketResult};
use rmeter_core::plan::model::WebSocketStep;

// ---------------------------------------------------------------------------
// WebSocket commands
// ---------------------------------------------------------------------------

/// Execute an ad-hoc WebSocket test scenario.
///
/// Connects to `url`, applies the given `headers` during the HTTP upgrade
/// handshake, then runs each `step` in order.  Returns per-step timing data
/// and a connected/error summary.
///
/// This command is intended for the interactive "WebSocket" tab in the UI.
/// Integration into the load-test engine virtual-user loop is a separate
/// future concern.
///
/// # Errors
///
/// Returns [`RmeterError::WebSocket`] when the connection or an individual
/// step fails (the error is also captured inside the returned
/// [`WebSocketResult`] for display purposes).
#[tauri::command]
pub async fn test_websocket(
    url: String,
    headers: HashMap<String, String>,
    steps: Vec<WebSocketStep>,
) -> Result<WebSocketResult, RmeterError> {
    let result = execute_websocket_scenario(&url, &headers, &steps).await;

    // Surface a top-level error if the connection itself failed.
    if !result.connected {
        let msg = result
            .error
            .clone()
            .unwrap_or_else(|| "Unknown WebSocket error".to_owned());
        return Err(RmeterError::WebSocket(msg));
    }

    Ok(result)
}

// ---------------------------------------------------------------------------
// GraphQL commands
// ---------------------------------------------------------------------------

/// Send a GraphQL query or mutation and return the raw HTTP response.
///
/// Builds a standard HTTP POST request with a JSON body containing `query`,
/// optional `variables`, and optional `operation_name`, then executes it
/// using the shared connection-pool [`HttpClient`].
///
/// Returns the same [`SendRequestOutput`] as `send_request` so the frontend
/// can display status, headers, body, and timing uniformly.
///
/// # Errors
///
/// Returns [`RmeterError::Validation`] when the GraphQL payload cannot be
/// serialized, or propagates any [`RmeterError::Http`] from the HTTP client.
#[tauri::command]
pub async fn send_graphql(
    url: String,
    query: String,
    variables: Option<serde_json::Value>,
    operation_name: Option<String>,
    headers: HashMap<String, String>,
    client: tauri::State<'_, HttpClient>,
) -> Result<SendRequestOutput, RmeterError> {
    let gql = GraphQLRequest {
        query,
        variables,
        operation_name,
    };

    let input = graphql_to_send_request_input(&url, &gql, &headers)
        .map_err(RmeterError::Validation)?;

    client.send(&input).await
}

/// Send a GraphQL introspection query and return the parsed schema as JSON.
///
/// Uses the standard `__schema` introspection document.  The response body is
/// parsed as JSON and returned so the frontend can render schema explorers or
/// autocomplete helpers.
///
/// # Errors
///
/// Returns [`RmeterError::Http`] on network failure, or
/// [`RmeterError::Serde`] when the server response is not valid JSON.
#[tauri::command]
pub async fn graphql_introspect(
    url: String,
    headers: HashMap<String, String>,
    client: tauri::State<'_, HttpClient>,
) -> Result<serde_json::Value, RmeterError> {
    let gql = introspection_request();

    let input = graphql_to_send_request_input(&url, &gql, &headers)
        .map_err(RmeterError::Validation)?;

    let output = client.send(&input).await?;

    // Parse the response body as JSON so the frontend gets a structured value.
    let schema: serde_json::Value = serde_json::from_str(&output.body)?;
    Ok(schema)
}
