# rmeter

A modern, open-source load testing tool built with Rust and React. Think JMeter, but fast, lightweight, and AI-ready.

rmeter provides both a desktop GUI application (via Tauri v2) and a Model Context Protocol (MCP) server that lets AI assistants like Claude create, configure, and run load tests autonomously.

## Features

- **Test Plan Builder** — Create and manage test plans with thread groups, HTTP requests, variables, assertions, and extractors
- **CSV Data Source** — Parameterize tests with CSV data (like JMeter's CSV Data Set Config), with configurable sharing modes and row recycling
- **Real-time Dashboard** — Live charts for response times, throughput, error rates, and active threads
- **Response Assertions** — Validate status codes, body content, headers, and response times
- **Response Extractors** — Capture values from responses (JSONPath, regex, headers) for request chaining
- **Variables** — Define global, plan, or thread-group scoped variables referenced as `${name}`
- **Templates** — Quick-start with built-in templates: REST API test, load test, stress test
- **Result History** — Store, compare, and export test results (CSV, JSON, HTML)
- **WebSocket Testing** — Define WebSocket test scenarios with connect, send, receive, and delay steps
- **GraphQL Support** — Dedicated GraphQL query/mutation testing with introspection
- **File I/O** — Save and load test plans as `.rmeter` files
- **MCP Server** — Full API for AI-driven load testing (29 tools)

## Architecture

```
rmeter/
├── crates/
│   ├── rmeter-core/     # Rust core: engine, plan model, results, HTTP client
│   ├── rmeter-tauri/    # Tauri v2 desktop app (Rust backend)
│   └── rmeter-mcp/      # MCP server (stdio JSON-RPC)
├── src/                 # React 18 + TypeScript frontend
│   ├── components/      # UI components (dashboard, plan editor, engine controls)
│   ├── stores/          # Zustand state management
│   ├── pages/           # Route pages
│   └── types/           # TypeScript type definitions
└── ...
```

## Getting Started

### Prerequisites

- Rust 1.75+
- Node.js 22+
- System libraries for Tauri (GTK3, WebKit2GTK on Linux)

### Development

```bash
# Install frontend dependencies
npm install

# Run in development mode (starts both Vite dev server and Tauri app)
cargo tauri dev
```

### Build

```bash
# Build the desktop app for your platform
cargo tauri build
```

### Run the MCP Server

```bash
# Build the MCP server binary
cargo build --release -p rmeter-mcp

# The binary is at target/release/rmeter-mcp
```

## MCP Server Configuration

The rmeter MCP server exposes 29 tools for AI assistants to create, configure, and execute load tests. Add it to your AI tool configuration:

### Claude Desktop

Add to `~/.config/Claude/claude_desktop_config.json` (Linux) or `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS):

```json
{
  "mcpServers": {
    "rmeter": {
      "command": "/path/to/rmeter-mcp",
      "args": []
    }
  }
}
```

### Claude Code

```bash
claude mcp add rmeter -- /path/to/rmeter-mcp
```

### MCP Tools Reference

| Tool | Description |
|------|-------------|
| **Plan Management** | |
| `create_test_plan` | Create a new test plan |
| `list_test_plans` | List all loaded test plans |
| `get_test_plan` | Get full plan details |
| `delete_test_plan` | Delete a test plan |
| `create_from_template` | Create from template (rest_api, load_test, stress_test) |
| `load_test_plan` | Load a .rmeter file from disk |
| `save_test_plan` | Save a test plan to disk |
| **Thread Groups** | |
| `add_thread_group` | Add a virtual user group |
| `update_thread_group` | Update thread group settings |
| `remove_thread_group` | Remove a thread group |
| **HTTP Requests** | |
| `add_request` | Add an HTTP request to a thread group |
| `update_request` | Update request method, URL, headers, body |
| `remove_request` | Remove a request |
| **Variables** | |
| `add_variable` | Add a plan variable (referenced as `${name}`) |
| `update_variable` | Update variable name, value, or scope |
| `remove_variable` | Remove a variable |
| **CSV Data Sources** | |
| `add_csv_data_source` | Add CSV data for parameterization |
| `update_csv_data_source` | Update sharing mode or recycle settings |
| `remove_csv_data_source` | Remove a CSV data source |
| **Assertions** | |
| `add_assertion` | Add a response assertion (status, body, time) |
| `update_assertion` | Update assertion rule |
| `remove_assertion` | Remove an assertion |
| **Extractors** | |
| `add_extractor` | Add a response data extractor |
| `update_extractor` | Update extractor expression |
| `remove_extractor` | Remove an extractor |
| **Test Execution** | |
| `run_test` | Execute a test plan (blocks until complete) |
| `get_test_status` | Check engine status (idle/running/complete) |
| `stop_test` | Gracefully stop a running test |
| `get_results` | Get live aggregated statistics |

### Example: AI-Driven Load Test

An AI assistant can use the MCP tools to run a complete load test:

```
1. create_test_plan(name: "API Load Test")
2. add_variable(plan_id, name: "base_url", value: "https://api.example.com")
3. add_thread_group(plan_id, name: "Users", num_threads: 50, ramp_up_seconds: 10,
                    loop_count: {type: "duration", seconds: 60})
4. add_request(plan_id, group_id, name: "GET /users", method: "GET",
              url: "${base_url}/users")
5. add_assertion(plan_id, group_id, request_id, name: "Status 200",
                rule: {type: "status_code", expected: 200})
6. add_csv_data_source(plan_id, name: "credentials",
                       csv_content: "username,password\nuser1,pass1\nuser2,pass2")
7. run_test(plan_id)  → returns full test summary with statistics
```

## License

[RLX Rixmerz License (RXL) v1.1](LICENSE) — Free core software with open ecosystem allowance.

The core software is always free. Extensions and plugins may be commercial.
