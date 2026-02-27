//! WebSocket ad-hoc testing client.
//!
//! Executes a sequence of [`WebSocketStep`]s against a WebSocket server and
//! returns per-step timing results.  Supports both plain (`ws://`) and TLS
//! (`wss://`) connections via `tokio-tungstenite` with native-TLS.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use base64::Engine as _;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::time::timeout;
use tokio_tungstenite::{
    connect_async_tls_with_config,
    tungstenite::{
        handshake::client::generate_key,
        http::Request as WsHttpRequest,
        Message,
    },
};

use crate::plan::model::WebSocketStep;

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

/// The outcome of running a single WebSocket step.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WebSocketStepResult {
    /// Zero-based index into the step list.
    pub step_index: usize,
    /// Human-readable name of the step kind (e.g. `"send_text"`).
    pub step_type: String,
    /// Wall-clock time the step took in milliseconds.
    pub elapsed_ms: u64,
    /// `true` when the step completed without error.
    pub success: bool,
    /// Received message payload, if the step was a `Receive`.
    pub message: Option<String>,
    /// Error description when `success` is `false`.
    pub error: Option<String>,
}

/// The aggregated result of running an entire WebSocket scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WebSocketResult {
    /// Per-step results in execution order.
    pub step_results: Vec<WebSocketStepResult>,
    /// Total wall-clock time for the entire scenario in milliseconds.
    pub total_elapsed_ms: u64,
    /// `true` when the initial WebSocket handshake succeeded.
    pub connected: bool,
    /// Top-level error description (e.g. connection failure); `None` on
    /// success or when only individual steps failed.
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Execute a WebSocket scenario.
///
/// Connects to `url` using the provided upgrade `headers`, then executes each
/// step in `steps` in order.  If a step fails the scenario stops early and
/// the remaining steps are not executed.
pub async fn execute_websocket_scenario(
    url: &str,
    headers: &HashMap<String, String>,
    steps: &[WebSocketStep],
) -> WebSocketResult {
    let overall_start = Instant::now();

    // Build the HTTP upgrade request so we can inject custom headers.
    let ws_request = match build_ws_request(url, headers) {
        Ok(r) => r,
        Err(e) => {
            return WebSocketResult {
                step_results: Vec::new(),
                total_elapsed_ms: overall_start.elapsed().as_millis() as u64,
                connected: false,
                error: Some(format!("Failed to build WebSocket request: {e}")),
            };
        }
    };

    // Establish the WebSocket connection.
    let connect_start = Instant::now();
    let (ws_stream, _response) =
        match connect_async_tls_with_config(ws_request, None, false, None).await {
            Ok(pair) => pair,
            Err(e) => {
                return WebSocketResult {
                    step_results: Vec::new(),
                    total_elapsed_ms: overall_start.elapsed().as_millis() as u64,
                    connected: false,
                    error: Some(format!("Connection failed: {e}")),
                };
            }
        };
    let connect_elapsed_ms = connect_start.elapsed().as_millis() as u64;

    let (mut sink, mut stream) = ws_stream.split();

    let mut step_results: Vec<WebSocketStepResult> = Vec::with_capacity(steps.len());

    // Record a synthetic "connect" step result so the caller sees the
    // handshake latency even when the first explicit step is not Connect.
    step_results.push(WebSocketStepResult {
        step_index: usize::MAX, // sentinel — not a user-defined step
        step_type: "connect".to_owned(),
        elapsed_ms: connect_elapsed_ms,
        success: true,
        message: None,
        error: None,
    });

    // Execute user-defined steps.
    let mut stopped_early = false;
    for (idx, step) in steps.iter().enumerate() {
        let step_start = Instant::now();

        let (step_type, success, message, error): (&str, bool, Option<String>, Option<String>) =
            match step {
                WebSocketStep::Connect { .. } => {
                    // The connection was already established above; this step
                    // is a no-op that re-reports the handshake latency.
                    ("connect", true, None, None)
                }

                WebSocketStep::SendText { message: text } => {
                    match sink.send(Message::Text(text.clone())).await {
                        Ok(()) => ("send_text", true, None, None),
                        Err(e) => ("send_text", false, None, Some(e.to_string())),
                    }
                }

                WebSocketStep::SendBinary { data } => {
                    match base64::engine::general_purpose::STANDARD.decode(data) {
                        Ok(bytes) => match sink.send(Message::Binary(bytes)).await {
                            Ok(()) => ("send_binary", true, None, None),
                            Err(e) => ("send_binary", false, None, Some(e.to_string())),
                        },
                        Err(e) => (
                            "send_binary",
                            false,
                            None,
                            Some(format!("Base64 decode error: {e}")),
                        ),
                    }
                }

                WebSocketStep::Receive { timeout_ms } => {
                    let dur = Duration::from_millis(*timeout_ms);
                    match timeout(dur, stream.next()).await {
                        Ok(Some(Ok(msg))) => {
                            let text = match msg {
                                Message::Text(t) => t.to_string(),
                                Message::Binary(b) => {
                                    base64::engine::general_purpose::STANDARD.encode(&b)
                                }
                                Message::Ping(_) => "<ping>".to_owned(),
                                Message::Pong(_) => "<pong>".to_owned(),
                                Message::Close(_) => "<close>".to_owned(),
                                Message::Frame(_) => "<raw-frame>".to_owned(),
                            };
                            ("receive", true, Some(text), None)
                        }
                        Ok(Some(Err(e))) => {
                            ("receive", false, None, Some(format!("Receive error: {e}")))
                        }
                        Ok(None) => (
                            "receive",
                            false,
                            None,
                            Some("Connection closed unexpectedly".to_owned()),
                        ),
                        Err(_elapsed) => (
                            "receive",
                            false,
                            None,
                            Some(format!("Timeout after {timeout_ms}ms")),
                        ),
                    }
                }

                WebSocketStep::Delay { duration_ms } => {
                    tokio::time::sleep(Duration::from_millis(*duration_ms)).await;
                    ("delay", true, None, None)
                }

                WebSocketStep::Close => {
                    let result = sink.send(Message::Close(None)).await;
                    match result {
                        Ok(()) => ("close", true, None, None),
                        Err(e) => ("close", false, None, Some(e.to_string())),
                    }
                }
            };

        let elapsed_ms = step_start.elapsed().as_millis() as u64;
        let step_failed = !success;

        step_results.push(WebSocketStepResult {
            step_index: idx,
            step_type: step_type.to_owned(),
            elapsed_ms,
            success,
            message,
            error,
        });

        if step_failed {
            stopped_early = true;
            break;
        }
    }

    // Best-effort close if we stopped early.
    if stopped_early {
        let _ = sink.send(Message::Close(None)).await;
    }

    WebSocketResult {
        step_results,
        total_elapsed_ms: overall_start.elapsed().as_millis() as u64,
        connected: true,
        error: None,
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Build a `tungstenite` HTTP upgrade request with the caller-supplied headers.
fn build_ws_request(
    url: &str,
    extra_headers: &HashMap<String, String>,
) -> Result<WsHttpRequest<()>, String> {
    let mut builder = WsHttpRequest::builder()
        .method("GET")
        .uri(url)
        .header("Host", extract_host(url)?)
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header("Sec-WebSocket-Key", generate_key());

    for (key, value) in extra_headers {
        builder = builder.header(key.as_str(), value.as_str());
    }

    builder.body(()).map_err(|e| e.to_string())
}

/// Extract the `host[:port]` component from a WebSocket URL for the `Host`
/// header, falling back to the raw URL if parsing fails.
fn extract_host(url: &str) -> Result<String, String> {
    // Strip scheme prefix before looking for the authority.
    let after_scheme = url
        .strip_prefix("wss://")
        .or_else(|| url.strip_prefix("ws://"))
        .unwrap_or(url);

    // Authority ends at '/' or '?' or end-of-string.
    let authority = after_scheme
        .split(['/', '?', '#'])
        .next()
        .unwrap_or(after_scheme);

    if authority.is_empty() {
        return Err(format!("Cannot extract host from URL: {url}"));
    }

    Ok(authority.to_owned())
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_host_plain() {
        assert_eq!(extract_host("ws://example.com/chat").unwrap(), "example.com");
    }

    #[test]
    fn extract_host_tls_with_port() {
        assert_eq!(
            extract_host("wss://echo.example.com:443/ws").unwrap(),
            "echo.example.com:443"
        );
    }

    #[test]
    fn extract_host_no_path() {
        assert_eq!(extract_host("ws://localhost:8080").unwrap(), "localhost:8080");
    }
}
