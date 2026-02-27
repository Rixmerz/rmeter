mod protocol;
mod server;
mod tools;

use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};

#[tokio::main]
async fn main() {
    // All logging goes to stderr — stdout is reserved for the MCP protocol.
    eprintln!("[rmeter-mcp] server starting (MCP protocol version 2024-11-05)");

    let server = server::McpServer::new();
    let stdin = io::stdin();
    let stdout = io::stdout();

    let mut reader = BufReader::new(stdin);
    let mut stdout = stdout;
    let mut line = String::new();

    loop {
        line.clear();
        match reader.read_line(&mut line).await {
            Ok(0) => {
                // EOF — client disconnected.
                eprintln!("[rmeter-mcp] stdin closed, shutting down");
                break;
            }
            Ok(_) => {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }

                let request: protocol::JsonRpcRequest = match serde_json::from_str(trimmed) {
                    Ok(req) => req,
                    Err(e) => {
                        let err = protocol::JsonRpcResponse::error(
                            serde_json::Value::Null,
                            -32700,
                            format!("Parse error: {e}"),
                        );
                        let out = serde_json::to_string(&err).unwrap_or_default();
                        let _ = stdout.write_all(out.as_bytes()).await;
                        let _ = stdout.write_all(b"\n").await;
                        let _ = stdout.flush().await;
                        continue;
                    }
                };

                if let Some(response) = server.handle_request(request).await {
                    let out = match serde_json::to_string(&response) {
                        Ok(s) => s,
                        Err(e) => {
                            eprintln!("[rmeter-mcp] Failed to serialize response: {e}");
                            continue;
                        }
                    };
                    if let Err(e) = stdout.write_all(out.as_bytes()).await {
                        eprintln!("[rmeter-mcp] stdout write error: {e}");
                        break;
                    }
                    if let Err(e) = stdout.write_all(b"\n").await {
                        eprintln!("[rmeter-mcp] stdout write error: {e}");
                        break;
                    }
                    if let Err(e) = stdout.flush().await {
                        eprintln!("[rmeter-mcp] stdout flush error: {e}");
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("[rmeter-mcp] stdin read error: {e}");
                break;
            }
        }
    }
}
