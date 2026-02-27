use std::sync::Arc;

use serde_json::Value;
use tokio::sync::Mutex;

use rmeter_core::plan::PlanManager;

use crate::protocol::{
    InitializeResult, JsonRpcRequest, JsonRpcResponse, ServerCapabilities, ServerInfo,
    ToolCallResult, ToolsCapability,
};
use crate::tools::{self, ToolState};

// ---------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------

pub struct McpServer {
    state: Arc<ToolState>,
}

impl McpServer {
    pub fn new() -> Self {
        Self {
            state: Arc::new(ToolState {
                plan_manager: Arc::new(Mutex::new(PlanManager::new())),
                engine_handle: Arc::new(Mutex::new(None)),
            }),
        }
    }

    /// Dispatch an incoming JSON-RPC request and return an optional response.
    ///
    /// Returns `None` for notifications (requests without an `id`), as the
    /// MCP spec does not require a response for them.
    pub async fn handle_request(&self, req: JsonRpcRequest) -> Option<JsonRpcResponse> {
        // Notifications have no id and never get a response.
        let id = match req.id {
            Some(id) => id,
            None => {
                eprintln!("[rmeter-mcp] Notification received: {}", req.method);
                return None;
            }
        };

        let result = match req.method.as_str() {
            "initialize" => self.handle_initialize(req.params),
            "tools/list" => self.handle_tools_list(),
            "tools/call" => self.handle_tools_call(req.params).await,
            other => Err((-32601, format!("Method not found: {other}"))),
        };

        Some(match result {
            Ok(value) => JsonRpcResponse::success(id, value),
            Err((code, msg)) => JsonRpcResponse::error(id, code, msg),
        })
    }

    // -----------------------------------------------------------------------
    // Method handlers
    // -----------------------------------------------------------------------

    fn handle_initialize(&self, _params: Option<Value>) -> Result<Value, (i32, String)> {
        let result = InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {}),
            },
            server_info: ServerInfo {
                name: "rmeter-mcp".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };
        serde_json::to_value(result).map_err(|e| (-32603, e.to_string()))
    }

    fn handle_tools_list(&self) -> Result<Value, (i32, String)> {
        let tool_defs = tools::all_tool_definitions();
        serde_json::to_value(serde_json::json!({ "tools": tool_defs }))
            .map_err(|e| (-32603, e.to_string()))
    }

    async fn handle_tools_call(&self, params: Option<Value>) -> Result<Value, (i32, String)> {
        let params = params.ok_or_else(|| (-32602, "Missing params for tools/call".to_string()))?;

        let name = params
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| (-32602, "Missing 'name' in tools/call params".to_string()))?
            .to_string();

        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or(Value::Object(Default::default()));

        eprintln!("[rmeter-mcp] Calling tool: {name}");

        let tool_result: ToolCallResult =
            tools::dispatch_tool(&name, arguments, &self.state).await;

        serde_json::to_value(tool_result).map_err(|e| (-32603, e.to_string()))
    }
}
