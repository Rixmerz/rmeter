use std::sync::Arc;

use serde_json::Value;
use tokio::sync::Mutex;
use uuid::Uuid;

use rmeter_core::engine::{self, AggregatorSnapshot, EngineConfig, EngineEvent, EngineStatus};
use rmeter_core::plan::manager::{HttpRequestUpdate, ThreadGroupUpdate};
use rmeter_core::plan::model::{CsvSharingMode, HttpMethod, LoopCount, RequestBody, VariableScope};
use rmeter_core::plan::{io as plan_io, templates, PlanManager};

use crate::protocol::{ContentBlock, ToolCallResult, ToolDefinition};

// ---------------------------------------------------------------------------
// State passed into every tool handler
// ---------------------------------------------------------------------------

pub struct ToolState {
    pub plan_manager: Arc<Mutex<PlanManager>>,
    pub engine_handle: Arc<Mutex<Option<engine::EngineHandle>>>,
}

// ---------------------------------------------------------------------------
// Tool definitions (advertised via tools/list)
// ---------------------------------------------------------------------------

pub fn all_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        // Plan CRUD
        create_test_plan_def(),
        list_test_plans_def(),
        get_test_plan_def(),
        delete_test_plan_def(),
        create_from_template_def(),
        // Thread group
        add_thread_group_def(),
        update_thread_group_def(),
        remove_thread_group_def(),
        // Request
        add_request_def(),
        update_request_def(),
        remove_request_def(),
        // Variables
        add_variable_def(),
        remove_variable_def(),
        update_variable_def(),
        // CSV data sources
        add_csv_data_source_def(),
        remove_csv_data_source_def(),
        update_csv_data_source_def(),
        // Assertions
        add_assertion_def(),
        remove_assertion_def(),
        update_assertion_def(),
        // Extractors
        add_extractor_def(),
        remove_extractor_def(),
        update_extractor_def(),
        // Engine
        run_test_def(),
        get_test_status_def(),
        stop_test_def(),
        get_results_def(),
        // File I/O
        load_test_plan_def(),
        save_test_plan_def(),
    ]
}

// ===========================================================================
// Tool definition functions
// ===========================================================================

fn create_test_plan_def() -> ToolDefinition {
    ToolDefinition {
        name: "create_test_plan".to_string(),
        description: "Create a new test plan for load testing. Returns the full plan including its generated ID.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "Name for the test plan"
                },
                "description": {
                    "type": "string",
                    "description": "Optional description of the test plan"
                }
            },
            "required": ["name"]
        }),
    }
}

fn list_test_plans_def() -> ToolDefinition {
    ToolDefinition {
        name: "list_test_plans".to_string(),
        description: "List all test plans currently loaded in memory. Returns an array of plan summaries.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {}
        }),
    }
}

fn get_test_plan_def() -> ToolDefinition {
    ToolDefinition {
        name: "get_test_plan".to_string(),
        description: "Get full details of a specific test plan, including all thread groups and requests.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "plan_id": {
                    "type": "string",
                    "description": "UUID of the test plan to retrieve"
                }
            },
            "required": ["plan_id"]
        }),
    }
}

fn delete_test_plan_def() -> ToolDefinition {
    ToolDefinition {
        name: "delete_test_plan".to_string(),
        description: "Delete a test plan from memory by its ID.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "plan_id": {
                    "type": "string",
                    "description": "UUID of the test plan to delete"
                }
            },
            "required": ["plan_id"]
        }),
    }
}

fn create_from_template_def() -> ToolDefinition {
    ToolDefinition {
        name: "create_from_template".to_string(),
        description: "Create a new test plan from a built-in template. Available templates: rest_api (REST API test with GET/POST), load_test (sustained load with 50 users), stress_test (ramping load up to 200 users).".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "template": {
                    "type": "string",
                    "description": "Template name: rest_api, load_test, or stress_test",
                    "enum": ["rest_api", "load_test", "stress_test"]
                }
            },
            "required": ["template"]
        }),
    }
}

fn add_thread_group_def() -> ToolDefinition {
    ToolDefinition {
        name: "add_thread_group".to_string(),
        description: "Add a thread group (virtual user group) to an existing test plan. Returns the new thread group.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "plan_id": {
                    "type": "string",
                    "description": "UUID of the test plan to add the thread group to"
                },
                "name": {
                    "type": "string",
                    "description": "Name for the thread group"
                },
                "num_threads": {
                    "type": "integer",
                    "description": "Number of concurrent virtual users (default: 1)",
                    "minimum": 1
                },
                "ramp_up_seconds": {
                    "type": "integer",
                    "description": "Time in seconds to ramp up to full thread count (default: 0)",
                    "minimum": 0
                },
                "loop_count": {
                    "type": "object",
                    "description": "How many times to loop: {\"type\":\"finite\",\"count\":N}, {\"type\":\"duration\",\"seconds\":N}, or {\"type\":\"infinite\"}",
                    "properties": {
                        "type": {
                            "type": "string",
                            "enum": ["finite", "duration", "infinite"]
                        }
                    }
                }
            },
            "required": ["plan_id", "name"]
        }),
    }
}

fn update_thread_group_def() -> ToolDefinition {
    ToolDefinition {
        name: "update_thread_group".to_string(),
        description: "Update properties of an existing thread group. All fields except plan_id and group_id are optional — only provided fields are changed.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "plan_id": {
                    "type": "string",
                    "description": "UUID of the test plan"
                },
                "group_id": {
                    "type": "string",
                    "description": "UUID of the thread group to update"
                },
                "name": {
                    "type": "string",
                    "description": "New name for the thread group"
                },
                "num_threads": {
                    "type": "integer",
                    "description": "New number of concurrent virtual users",
                    "minimum": 1
                },
                "ramp_up_seconds": {
                    "type": "integer",
                    "description": "New ramp-up time in seconds",
                    "minimum": 0
                },
                "loop_count": {
                    "type": "object",
                    "description": "New loop configuration"
                },
                "enabled": {
                    "type": "boolean",
                    "description": "Enable or disable the thread group"
                }
            },
            "required": ["plan_id", "group_id"]
        }),
    }
}

fn remove_thread_group_def() -> ToolDefinition {
    ToolDefinition {
        name: "remove_thread_group".to_string(),
        description: "Remove a thread group from a test plan.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "plan_id": {
                    "type": "string",
                    "description": "UUID of the test plan"
                },
                "group_id": {
                    "type": "string",
                    "description": "UUID of the thread group to remove"
                }
            },
            "required": ["plan_id", "group_id"]
        }),
    }
}

fn add_request_def() -> ToolDefinition {
    ToolDefinition {
        name: "add_request".to_string(),
        description: "Add an HTTP request to a thread group within a test plan. Returns the new request.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "plan_id": {
                    "type": "string",
                    "description": "UUID of the test plan"
                },
                "group_id": {
                    "type": "string",
                    "description": "UUID of the thread group"
                },
                "name": {
                    "type": "string",
                    "description": "Name for the request"
                },
                "method": {
                    "type": "string",
                    "description": "HTTP method (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS)",
                    "enum": ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"]
                },
                "url": {
                    "type": "string",
                    "description": "Target URL for the request"
                },
                "headers": {
                    "type": "object",
                    "description": "HTTP headers as key-value pairs",
                    "additionalProperties": {
                        "type": "string"
                    }
                },
                "body": {
                    "type": "object",
                    "description": "Request body: {\"type\":\"json\",\"content\":\"...\"},  {\"type\":\"raw\",\"content\":\"...\"}, or {\"type\":\"xml\",\"content\":\"...\"}"
                }
            },
            "required": ["plan_id", "group_id", "name"]
        }),
    }
}

fn update_request_def() -> ToolDefinition {
    ToolDefinition {
        name: "update_request".to_string(),
        description: "Update properties of an existing HTTP request. All fields except plan_id, group_id, and request_id are optional.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "plan_id": {
                    "type": "string",
                    "description": "UUID of the test plan"
                },
                "group_id": {
                    "type": "string",
                    "description": "UUID of the thread group"
                },
                "request_id": {
                    "type": "string",
                    "description": "UUID of the request to update"
                },
                "name": {
                    "type": "string",
                    "description": "New name for the request"
                },
                "method": {
                    "type": "string",
                    "description": "New HTTP method",
                    "enum": ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"]
                },
                "url": {
                    "type": "string",
                    "description": "New target URL"
                },
                "headers": {
                    "type": "object",
                    "description": "New HTTP headers (replaces all existing headers)",
                    "additionalProperties": { "type": "string" }
                },
                "body": {
                    "type": "object",
                    "description": "New request body (or null to clear)"
                },
                "enabled": {
                    "type": "boolean",
                    "description": "Enable or disable the request"
                }
            },
            "required": ["plan_id", "group_id", "request_id"]
        }),
    }
}

fn remove_request_def() -> ToolDefinition {
    ToolDefinition {
        name: "remove_request".to_string(),
        description: "Remove an HTTP request from a thread group.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "plan_id": {
                    "type": "string",
                    "description": "UUID of the test plan"
                },
                "group_id": {
                    "type": "string",
                    "description": "UUID of the thread group"
                },
                "request_id": {
                    "type": "string",
                    "description": "UUID of the request to remove"
                }
            },
            "required": ["plan_id", "group_id", "request_id"]
        }),
    }
}

fn add_variable_def() -> ToolDefinition {
    ToolDefinition {
        name: "add_variable".to_string(),
        description: "Add a variable to a test plan. Variables can be referenced as ${name} in URLs, headers, and request bodies.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "plan_id": {
                    "type": "string",
                    "description": "UUID of the test plan"
                },
                "name": {
                    "type": "string",
                    "description": "Variable name (referenced as ${name} in requests)"
                },
                "value": {
                    "type": "string",
                    "description": "Variable value"
                },
                "scope": {
                    "type": "string",
                    "description": "Variable scope: global, plan (default), or thread_group",
                    "enum": ["global", "plan", "thread_group"]
                }
            },
            "required": ["plan_id", "name", "value"]
        }),
    }
}

fn remove_variable_def() -> ToolDefinition {
    ToolDefinition {
        name: "remove_variable".to_string(),
        description: "Remove a variable from a test plan.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "plan_id": {
                    "type": "string",
                    "description": "UUID of the test plan"
                },
                "variable_id": {
                    "type": "string",
                    "description": "UUID of the variable to remove"
                }
            },
            "required": ["plan_id", "variable_id"]
        }),
    }
}

fn update_variable_def() -> ToolDefinition {
    ToolDefinition {
        name: "update_variable".to_string(),
        description: "Update a variable's name, value, or scope. Only provided fields are changed.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "plan_id": {
                    "type": "string",
                    "description": "UUID of the test plan"
                },
                "variable_id": {
                    "type": "string",
                    "description": "UUID of the variable to update"
                },
                "name": {
                    "type": "string",
                    "description": "New variable name"
                },
                "value": {
                    "type": "string",
                    "description": "New variable value"
                },
                "scope": {
                    "type": "string",
                    "description": "New scope: global, plan, or thread_group",
                    "enum": ["global", "plan", "thread_group"]
                }
            },
            "required": ["plan_id", "variable_id"]
        }),
    }
}

fn add_csv_data_source_def() -> ToolDefinition {
    ToolDefinition {
        name: "add_csv_data_source".to_string(),
        description: "Add a CSV data source to a test plan. The first row is used as headers (column names), which become variables available as ${column_name}. Each virtual user iteration reads the next row.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "plan_id": {
                    "type": "string",
                    "description": "UUID of the test plan"
                },
                "name": {
                    "type": "string",
                    "description": "Name for the CSV data source"
                },
                "csv_content": {
                    "type": "string",
                    "description": "Raw CSV content with header row, e.g. \"username,password\\nuser1,pass1\\nuser2,pass2\""
                },
                "delimiter": {
                    "type": "string",
                    "description": "Column delimiter character (default: comma). Use \\t for tab, ; for semicolon, etc."
                }
            },
            "required": ["plan_id", "name", "csv_content"]
        }),
    }
}

fn remove_csv_data_source_def() -> ToolDefinition {
    ToolDefinition {
        name: "remove_csv_data_source".to_string(),
        description: "Remove a CSV data source from a test plan.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "plan_id": {
                    "type": "string",
                    "description": "UUID of the test plan"
                },
                "source_id": {
                    "type": "string",
                    "description": "UUID of the CSV data source to remove"
                }
            },
            "required": ["plan_id", "source_id"]
        }),
    }
}

fn update_csv_data_source_def() -> ToolDefinition {
    ToolDefinition {
        name: "update_csv_data_source".to_string(),
        description: "Update a CSV data source's metadata (name, sharing mode, recycle flag).".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "plan_id": {
                    "type": "string",
                    "description": "UUID of the test plan"
                },
                "source_id": {
                    "type": "string",
                    "description": "UUID of the CSV data source to update"
                },
                "name": {
                    "type": "string",
                    "description": "New name"
                },
                "sharing_mode": {
                    "type": "string",
                    "description": "Row distribution mode: all_threads (shared counter) or per_thread (independent counters)",
                    "enum": ["all_threads", "per_thread"]
                },
                "recycle": {
                    "type": "boolean",
                    "description": "Whether to wrap around to the first row when all rows are consumed"
                }
            },
            "required": ["plan_id", "source_id"]
        }),
    }
}

fn add_assertion_def() -> ToolDefinition {
    ToolDefinition {
        name: "add_assertion".to_string(),
        description: "Add a response assertion to an HTTP request. Assertions validate response properties (status code, body content, headers, response time).".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "plan_id": {
                    "type": "string",
                    "description": "UUID of the test plan"
                },
                "group_id": {
                    "type": "string",
                    "description": "UUID of the thread group"
                },
                "request_id": {
                    "type": "string",
                    "description": "UUID of the request to add the assertion to"
                },
                "name": {
                    "type": "string",
                    "description": "Name for the assertion (e.g. 'Status is 200')"
                },
                "rule": {
                    "type": "object",
                    "description": "Assertion rule as JSON. Examples: {\"type\":\"status_code\",\"expected\":200}, {\"type\":\"body_contains\",\"value\":\"success\"}, {\"type\":\"response_time_ms\",\"max\":500}"
                }
            },
            "required": ["plan_id", "group_id", "request_id", "name", "rule"]
        }),
    }
}

fn remove_assertion_def() -> ToolDefinition {
    ToolDefinition {
        name: "remove_assertion".to_string(),
        description: "Remove an assertion from an HTTP request.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "plan_id": {
                    "type": "string",
                    "description": "UUID of the test plan"
                },
                "group_id": {
                    "type": "string",
                    "description": "UUID of the thread group"
                },
                "request_id": {
                    "type": "string",
                    "description": "UUID of the request"
                },
                "assertion_id": {
                    "type": "string",
                    "description": "UUID of the assertion to remove"
                }
            },
            "required": ["plan_id", "group_id", "request_id", "assertion_id"]
        }),
    }
}

fn update_assertion_def() -> ToolDefinition {
    ToolDefinition {
        name: "update_assertion".to_string(),
        description: "Update an assertion's name or rule. Only provided fields are changed.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "plan_id": {
                    "type": "string",
                    "description": "UUID of the test plan"
                },
                "group_id": {
                    "type": "string",
                    "description": "UUID of the thread group"
                },
                "request_id": {
                    "type": "string",
                    "description": "UUID of the request"
                },
                "assertion_id": {
                    "type": "string",
                    "description": "UUID of the assertion to update"
                },
                "name": {
                    "type": "string",
                    "description": "New assertion name"
                },
                "rule": {
                    "type": "object",
                    "description": "New assertion rule"
                }
            },
            "required": ["plan_id", "group_id", "request_id", "assertion_id"]
        }),
    }
}

fn add_extractor_def() -> ToolDefinition {
    ToolDefinition {
        name: "add_extractor".to_string(),
        description: "Add a response extractor to an HTTP request. Extractors capture values from responses and store them as variables for use in subsequent requests.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "plan_id": {
                    "type": "string",
                    "description": "UUID of the test plan"
                },
                "group_id": {
                    "type": "string",
                    "description": "UUID of the thread group"
                },
                "request_id": {
                    "type": "string",
                    "description": "UUID of the request to add the extractor to"
                },
                "name": {
                    "type": "string",
                    "description": "Name for the extractor (e.g. 'Extract auth token')"
                },
                "variable": {
                    "type": "string",
                    "description": "Variable name to store the extracted value in (referenced as ${variable})"
                },
                "expression": {
                    "type": "object",
                    "description": "Extraction expression as JSON. Examples: {\"type\":\"json_path\",\"path\":\"$.data.token\"}, {\"type\":\"regex\",\"pattern\":\"token=(\\w+)\"}, {\"type\":\"header\",\"name\":\"X-Request-Id\"}"
                }
            },
            "required": ["plan_id", "group_id", "request_id", "name", "variable", "expression"]
        }),
    }
}

fn remove_extractor_def() -> ToolDefinition {
    ToolDefinition {
        name: "remove_extractor".to_string(),
        description: "Remove an extractor from an HTTP request.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "plan_id": {
                    "type": "string",
                    "description": "UUID of the test plan"
                },
                "group_id": {
                    "type": "string",
                    "description": "UUID of the thread group"
                },
                "request_id": {
                    "type": "string",
                    "description": "UUID of the request"
                },
                "extractor_id": {
                    "type": "string",
                    "description": "UUID of the extractor to remove"
                }
            },
            "required": ["plan_id", "group_id", "request_id", "extractor_id"]
        }),
    }
}

fn update_extractor_def() -> ToolDefinition {
    ToolDefinition {
        name: "update_extractor".to_string(),
        description: "Update an extractor's name, variable, or expression. Only provided fields are changed.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "plan_id": {
                    "type": "string",
                    "description": "UUID of the test plan"
                },
                "group_id": {
                    "type": "string",
                    "description": "UUID of the thread group"
                },
                "request_id": {
                    "type": "string",
                    "description": "UUID of the request"
                },
                "extractor_id": {
                    "type": "string",
                    "description": "UUID of the extractor to update"
                },
                "name": {
                    "type": "string",
                    "description": "New extractor name"
                },
                "variable": {
                    "type": "string",
                    "description": "New variable name"
                },
                "expression": {
                    "type": "object",
                    "description": "New extraction expression"
                }
            },
            "required": ["plan_id", "group_id", "request_id", "extractor_id"]
        }),
    }
}

fn run_test_def() -> ToolDefinition {
    ToolDefinition {
        name: "run_test".to_string(),
        description: "Execute a test plan and wait for it to complete. Returns the final TestSummary with all statistics. NOTE: This call blocks until the test finishes.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "plan_id": {
                    "type": "string",
                    "description": "UUID of the test plan to execute"
                }
            },
            "required": ["plan_id"]
        }),
    }
}

fn get_test_status_def() -> ToolDefinition {
    ToolDefinition {
        name: "get_test_status".to_string(),
        description: "Get the current status of the test engine (idle, running, stopping, completed, error).".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {}
        }),
    }
}

fn stop_test_def() -> ToolDefinition {
    ToolDefinition {
        name: "stop_test".to_string(),
        description: "Gracefully stop a currently running test. The engine will finish in-flight requests and emit a final summary.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {}
        }),
    }
}

fn get_results_def() -> ToolDefinition {
    ToolDefinition {
        name: "get_results".to_string(),
        description: "Get a live snapshot of aggregated statistics from a running (or recently finished) test.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {}
        }),
    }
}

fn load_test_plan_def() -> ToolDefinition {
    ToolDefinition {
        name: "load_test_plan".to_string(),
        description: "Load a test plan from a .rmeter file on disk. Returns the loaded plan.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Absolute or relative path to the .rmeter file"
                }
            },
            "required": ["path"]
        }),
    }
}

fn save_test_plan_def() -> ToolDefinition {
    ToolDefinition {
        name: "save_test_plan".to_string(),
        description: "Save a test plan to a .rmeter file on disk as pretty-printed JSON.".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "plan_id": {
                    "type": "string",
                    "description": "UUID of the test plan to save"
                },
                "path": {
                    "type": "string",
                    "description": "Destination file path (e.g. ./my-plan.rmeter)"
                }
            },
            "required": ["plan_id", "path"]
        }),
    }
}

// ---------------------------------------------------------------------------
// Tool dispatch
// ---------------------------------------------------------------------------

pub async fn dispatch_tool(
    name: &str,
    args: Value,
    state: &ToolState,
) -> ToolCallResult {
    match name {
        // Plan CRUD
        "create_test_plan" => handle_create_test_plan(args, state).await,
        "list_test_plans" => handle_list_test_plans(state).await,
        "get_test_plan" => handle_get_test_plan(args, state).await,
        "delete_test_plan" => handle_delete_test_plan(args, state).await,
        "create_from_template" => handle_create_from_template(args, state).await,
        // Thread group
        "add_thread_group" => handle_add_thread_group(args, state).await,
        "update_thread_group" => handle_update_thread_group(args, state).await,
        "remove_thread_group" => handle_remove_thread_group(args, state).await,
        // Request
        "add_request" => handle_add_request(args, state).await,
        "update_request" => handle_update_request(args, state).await,
        "remove_request" => handle_remove_request(args, state).await,
        // Variables
        "add_variable" => handle_add_variable(args, state).await,
        "remove_variable" => handle_remove_variable(args, state).await,
        "update_variable" => handle_update_variable(args, state).await,
        // CSV data sources
        "add_csv_data_source" => handle_add_csv_data_source(args, state).await,
        "remove_csv_data_source" => handle_remove_csv_data_source(args, state).await,
        "update_csv_data_source" => handle_update_csv_data_source(args, state).await,
        // Assertions
        "add_assertion" => handle_add_assertion(args, state).await,
        "remove_assertion" => handle_remove_assertion(args, state).await,
        "update_assertion" => handle_update_assertion(args, state).await,
        // Extractors
        "add_extractor" => handle_add_extractor(args, state).await,
        "remove_extractor" => handle_remove_extractor(args, state).await,
        "update_extractor" => handle_update_extractor(args, state).await,
        // Engine
        "run_test" => handle_run_test(args, state).await,
        "get_test_status" => handle_get_test_status(state).await,
        "stop_test" => handle_stop_test(state).await,
        "get_results" => handle_get_results(state).await,
        // File I/O
        "load_test_plan" => handle_load_test_plan(args, state).await,
        "save_test_plan" => handle_save_test_plan(args, state).await,
        unknown => tool_error(format!("Unknown tool: {unknown}")),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn tool_ok(text: String) -> ToolCallResult {
    ToolCallResult {
        content: vec![ContentBlock::Text { text }],
        is_error: None,
    }
}

fn tool_error(message: String) -> ToolCallResult {
    ToolCallResult {
        content: vec![ContentBlock::Text { text: message }],
        is_error: Some(true),
    }
}

fn require_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("Missing required argument: {key}"))
}

fn parse_uuid(s: &str, label: &str) -> Result<Uuid, String> {
    Uuid::parse_str(s).map_err(|e| format!("Invalid {label}: {e}"))
}

fn json_ok<T: serde::Serialize>(value: &T) -> ToolCallResult {
    match serde_json::to_string_pretty(value) {
        Ok(json) => tool_ok(json),
        Err(e) => tool_error(format!("Serialization error: {e}")),
    }
}

// ---------------------------------------------------------------------------
// Plan CRUD handlers
// ---------------------------------------------------------------------------

async fn handle_create_test_plan(args: Value, state: &ToolState) -> ToolCallResult {
    let name = match require_str(&args, "name") {
        Ok(v) => v.to_string(),
        Err(e) => return tool_error(e),
    };

    let description = args
        .get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let mut mgr = state.plan_manager.lock().await;
    let plan_id = mgr.create_plan(name);

    if !description.is_empty() {
        if let Some(plan) = mgr.get_plan_mut(&plan_id) {
            plan.description = description;
        }
    }

    match mgr.get_plan(&plan_id) {
        Some(plan) => json_ok(plan),
        None => tool_error(format!("Plan {} not found after creation", plan_id)),
    }
}

async fn handle_list_test_plans(state: &ToolState) -> ToolCallResult {
    let mgr = state.plan_manager.lock().await;
    json_ok(&mgr.list_plans())
}

async fn handle_get_test_plan(args: Value, state: &ToolState) -> ToolCallResult {
    let plan_id = match require_str(&args, "plan_id").and_then(|s| parse_uuid(s, "plan_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };

    let mgr = state.plan_manager.lock().await;
    match mgr.get_plan(&plan_id) {
        Some(plan) => json_ok(plan),
        None => tool_error(format!("Plan not found: {plan_id}")),
    }
}

async fn handle_delete_test_plan(args: Value, state: &ToolState) -> ToolCallResult {
    let plan_id = match require_str(&args, "plan_id").and_then(|s| parse_uuid(s, "plan_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };

    let mut mgr = state.plan_manager.lock().await;
    if mgr.delete_plan(&plan_id) {
        tool_ok(format!("Plan {plan_id} deleted"))
    } else {
        tool_error(format!("Plan not found: {plan_id}"))
    }
}

async fn handle_create_from_template(args: Value, state: &ToolState) -> ToolCallResult {
    let template = match require_str(&args, "template") {
        Ok(v) => v.to_string(),
        Err(e) => return tool_error(e),
    };

    let plan = match template.as_str() {
        "rest_api" => templates::rest_api_test(),
        "load_test" => templates::load_test(),
        "stress_test" => templates::stress_test(),
        other => return tool_error(format!(
            "Unknown template '{}'. Valid options: rest_api, load_test, stress_test",
            other
        )),
    };

    let json = json_ok(&plan);
    let mut mgr = state.plan_manager.lock().await;
    mgr.add_plan(plan);
    json
}

// ---------------------------------------------------------------------------
// Thread group handlers
// ---------------------------------------------------------------------------

async fn handle_add_thread_group(args: Value, state: &ToolState) -> ToolCallResult {
    let plan_id = match require_str(&args, "plan_id").and_then(|s| parse_uuid(s, "plan_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let name = match require_str(&args, "name") {
        Ok(v) => v.to_string(),
        Err(e) => return tool_error(e),
    };

    let num_threads = args.get("num_threads").and_then(|v| v.as_u64()).map(|v| v as u32);
    let ramp_up_seconds = args.get("ramp_up_seconds").and_then(|v| v.as_u64()).map(|v| v as u32);
    let loop_count: Option<LoopCount> = args.get("loop_count").and_then(|v| {
        serde_json::from_value(v.clone()).ok()
    });

    let mut mgr = state.plan_manager.lock().await;
    let group_id = match mgr.add_thread_group(&plan_id, name) {
        Ok(id) => id,
        Err(e) => return tool_error(e.to_string()),
    };

    if num_threads.is_some() || ramp_up_seconds.is_some() || loop_count.is_some() {
        let update = ThreadGroupUpdate {
            name: None,
            num_threads,
            ramp_up_seconds,
            loop_count,
            enabled: None,
        };
        if let Err(e) = mgr.update_thread_group(&plan_id, &group_id, update) {
            return tool_error(format!("Thread group created but update failed: {e}"));
        }
    }

    let plan = match mgr.get_plan(&plan_id) {
        Some(p) => p,
        None => return tool_error(format!("Plan {} not found after adding thread group", plan_id)),
    };
    match plan.thread_groups.iter().find(|tg| tg.id == group_id) {
        Some(tg) => json_ok(tg),
        None => tool_error(format!("Thread group {} not found after creation", group_id)),
    }
}

async fn handle_update_thread_group(args: Value, state: &ToolState) -> ToolCallResult {
    let plan_id = match require_str(&args, "plan_id").and_then(|s| parse_uuid(s, "plan_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let group_id = match require_str(&args, "group_id").and_then(|s| parse_uuid(s, "group_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };

    let update = ThreadGroupUpdate {
        name: args.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()),
        num_threads: args.get("num_threads").and_then(|v| v.as_u64()).map(|v| v as u32),
        ramp_up_seconds: args.get("ramp_up_seconds").and_then(|v| v.as_u64()).map(|v| v as u32),
        loop_count: args.get("loop_count").and_then(|v| serde_json::from_value(v.clone()).ok()),
        enabled: args.get("enabled").and_then(|v| v.as_bool()),
    };

    let mut mgr = state.plan_manager.lock().await;
    match mgr.update_thread_group(&plan_id, &group_id, update) {
        Ok(tg) => json_ok(tg),
        Err(e) => tool_error(e.to_string()),
    }
}

async fn handle_remove_thread_group(args: Value, state: &ToolState) -> ToolCallResult {
    let plan_id = match require_str(&args, "plan_id").and_then(|s| parse_uuid(s, "plan_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let group_id = match require_str(&args, "group_id").and_then(|s| parse_uuid(s, "group_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };

    let mut mgr = state.plan_manager.lock().await;
    match mgr.remove_thread_group(&plan_id, &group_id) {
        Ok(()) => tool_ok(format!("Thread group {group_id} removed from plan {plan_id}")),
        Err(e) => tool_error(e.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Request handlers
// ---------------------------------------------------------------------------

async fn handle_add_request(args: Value, state: &ToolState) -> ToolCallResult {
    let plan_id = match require_str(&args, "plan_id").and_then(|s| parse_uuid(s, "plan_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let group_id = match require_str(&args, "group_id").and_then(|s| parse_uuid(s, "group_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let name = match require_str(&args, "name") {
        Ok(v) => v.to_string(),
        Err(e) => return tool_error(e),
    };

    let method: Option<HttpMethod> = args.get("method").and_then(|v| {
        serde_json::from_value(v.clone()).ok()
    });
    let url = args.get("url").and_then(|v| v.as_str()).map(|s| s.to_string());
    let headers = args.get("headers").and_then(|v| {
        serde_json::from_value(v.clone()).ok()
    });
    let body: Option<Option<RequestBody>> = args.get("body").map(|v| {
        serde_json::from_value(v.clone()).ok()
    });

    let mut mgr = state.plan_manager.lock().await;
    let request_id = match mgr.add_request(&plan_id, &group_id, name) {
        Ok(id) => id,
        Err(e) => return tool_error(e.to_string()),
    };

    if method.is_some() || url.is_some() || headers.is_some() || body.is_some() {
        let update = HttpRequestUpdate {
            name: None,
            method,
            url,
            headers,
            body,
            enabled: None,
        };
        if let Err(e) = mgr.update_request(&plan_id, &group_id, &request_id, update) {
            return tool_error(format!("Request created but update failed: {e}"));
        }
    }

    let plan = match mgr.get_plan(&plan_id) {
        Some(p) => p,
        None => return tool_error(format!("Plan {} not found after adding request", plan_id)),
    };
    let tg = match plan.thread_groups.iter().find(|tg| tg.id == group_id) {
        Some(t) => t,
        None => return tool_error(format!("Thread group {} not found", group_id)),
    };
    match tg.requests.iter().find(|r| r.id == request_id) {
        Some(req) => json_ok(req),
        None => tool_error(format!("Request {} not found after creation", request_id)),
    }
}

async fn handle_update_request(args: Value, state: &ToolState) -> ToolCallResult {
    let plan_id = match require_str(&args, "plan_id").and_then(|s| parse_uuid(s, "plan_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let group_id = match require_str(&args, "group_id").and_then(|s| parse_uuid(s, "group_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let request_id = match require_str(&args, "request_id").and_then(|s| parse_uuid(s, "request_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };

    let update = HttpRequestUpdate {
        name: args.get("name").and_then(|v| v.as_str()).map(|s| s.to_string()),
        method: args.get("method").and_then(|v| serde_json::from_value(v.clone()).ok()),
        url: args.get("url").and_then(|v| v.as_str()).map(|s| s.to_string()),
        headers: args.get("headers").and_then(|v| serde_json::from_value(v.clone()).ok()),
        body: args.get("body").map(|v| serde_json::from_value(v.clone()).ok()),
        enabled: args.get("enabled").and_then(|v| v.as_bool()),
    };

    let mut mgr = state.plan_manager.lock().await;
    match mgr.update_request(&plan_id, &group_id, &request_id, update) {
        Ok(req) => json_ok(req),
        Err(e) => tool_error(e.to_string()),
    }
}

async fn handle_remove_request(args: Value, state: &ToolState) -> ToolCallResult {
    let plan_id = match require_str(&args, "plan_id").and_then(|s| parse_uuid(s, "plan_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let group_id = match require_str(&args, "group_id").and_then(|s| parse_uuid(s, "group_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let request_id = match require_str(&args, "request_id").and_then(|s| parse_uuid(s, "request_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };

    let mut mgr = state.plan_manager.lock().await;
    match mgr.remove_request(&plan_id, &group_id, &request_id) {
        Ok(()) => tool_ok(format!("Request {request_id} removed")),
        Err(e) => tool_error(e.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Variable handlers
// ---------------------------------------------------------------------------

async fn handle_add_variable(args: Value, state: &ToolState) -> ToolCallResult {
    let plan_id = match require_str(&args, "plan_id").and_then(|s| parse_uuid(s, "plan_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let name = match require_str(&args, "name") {
        Ok(v) => v.to_string(),
        Err(e) => return tool_error(e),
    };
    let value = match require_str(&args, "value") {
        Ok(v) => v.to_string(),
        Err(e) => return tool_error(e),
    };

    let scope: VariableScope = args
        .get("scope")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let mut mgr = state.plan_manager.lock().await;
    match mgr.add_variable(&plan_id, name, value, scope) {
        Ok(variable) => json_ok(&variable),
        Err(e) => tool_error(e.to_string()),
    }
}

async fn handle_remove_variable(args: Value, state: &ToolState) -> ToolCallResult {
    let plan_id = match require_str(&args, "plan_id").and_then(|s| parse_uuid(s, "plan_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let variable_id = match require_str(&args, "variable_id").and_then(|s| parse_uuid(s, "variable_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };

    let mut mgr = state.plan_manager.lock().await;
    match mgr.remove_variable(&plan_id, &variable_id) {
        Ok(()) => tool_ok(format!("Variable {variable_id} removed")),
        Err(e) => tool_error(e.to_string()),
    }
}

async fn handle_update_variable(args: Value, state: &ToolState) -> ToolCallResult {
    let plan_id = match require_str(&args, "plan_id").and_then(|s| parse_uuid(s, "plan_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let variable_id = match require_str(&args, "variable_id").and_then(|s| parse_uuid(s, "variable_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };

    let name = args.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
    let value = args.get("value").and_then(|v| v.as_str()).map(|s| s.to_string());
    let scope: Option<VariableScope> = args.get("scope").and_then(|v| serde_json::from_value(v.clone()).ok());

    let mut mgr = state.plan_manager.lock().await;
    match mgr.update_variable(&plan_id, &variable_id, name, value, scope) {
        Ok(variable) => json_ok(&variable),
        Err(e) => tool_error(e.to_string()),
    }
}

// ---------------------------------------------------------------------------
// CSV data source handlers
// ---------------------------------------------------------------------------

async fn handle_add_csv_data_source(args: Value, state: &ToolState) -> ToolCallResult {
    let plan_id = match require_str(&args, "plan_id").and_then(|s| parse_uuid(s, "plan_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let name = match require_str(&args, "name") {
        Ok(v) => v.to_string(),
        Err(e) => return tool_error(e),
    };
    let csv_content = match require_str(&args, "csv_content") {
        Ok(v) => v.to_string(),
        Err(e) => return tool_error(e),
    };

    let delimiter = args
        .get("delimiter")
        .and_then(|v| v.as_str())
        .and_then(|d| d.as_bytes().first().copied())
        .unwrap_or(b',');

    let mut mgr = state.plan_manager.lock().await;
    match mgr.add_csv_data_source(&plan_id, name, csv_content, Some(delimiter)) {
        Ok(source) => json_ok(&source),
        Err(e) => tool_error(e.to_string()),
    }
}

async fn handle_remove_csv_data_source(args: Value, state: &ToolState) -> ToolCallResult {
    let plan_id = match require_str(&args, "plan_id").and_then(|s| parse_uuid(s, "plan_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let source_id = match require_str(&args, "source_id").and_then(|s| parse_uuid(s, "source_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };

    let mut mgr = state.plan_manager.lock().await;
    match mgr.remove_csv_data_source(&plan_id, &source_id) {
        Ok(()) => tool_ok(format!("CSV data source {source_id} removed")),
        Err(e) => tool_error(e.to_string()),
    }
}

async fn handle_update_csv_data_source(args: Value, state: &ToolState) -> ToolCallResult {
    let plan_id = match require_str(&args, "plan_id").and_then(|s| parse_uuid(s, "plan_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let source_id = match require_str(&args, "source_id").and_then(|s| parse_uuid(s, "source_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };

    let name = args.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
    let sharing_mode: Option<CsvSharingMode> = args
        .get("sharing_mode")
        .and_then(|v| serde_json::from_value(v.clone()).ok());
    let recycle = args.get("recycle").and_then(|v| v.as_bool());

    let mut mgr = state.plan_manager.lock().await;
    match mgr.update_csv_data_source(&plan_id, &source_id, name, sharing_mode, recycle) {
        Ok(source) => json_ok(&source),
        Err(e) => tool_error(e.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Assertion handlers
// ---------------------------------------------------------------------------

async fn handle_add_assertion(args: Value, state: &ToolState) -> ToolCallResult {
    let plan_id = match require_str(&args, "plan_id").and_then(|s| parse_uuid(s, "plan_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let group_id = match require_str(&args, "group_id").and_then(|s| parse_uuid(s, "group_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let request_id = match require_str(&args, "request_id").and_then(|s| parse_uuid(s, "request_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let name = match require_str(&args, "name") {
        Ok(v) => v.to_string(),
        Err(e) => return tool_error(e),
    };
    let rule = match args.get("rule") {
        Some(v) => v.clone(),
        None => return tool_error("Missing required argument: rule".to_string()),
    };

    let mut mgr = state.plan_manager.lock().await;
    match mgr.add_assertion(&plan_id, &group_id, &request_id, name, rule) {
        Ok(assertion) => json_ok(&assertion),
        Err(e) => tool_error(e.to_string()),
    }
}

async fn handle_remove_assertion(args: Value, state: &ToolState) -> ToolCallResult {
    let plan_id = match require_str(&args, "plan_id").and_then(|s| parse_uuid(s, "plan_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let group_id = match require_str(&args, "group_id").and_then(|s| parse_uuid(s, "group_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let request_id = match require_str(&args, "request_id").and_then(|s| parse_uuid(s, "request_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let assertion_id = match require_str(&args, "assertion_id").and_then(|s| parse_uuid(s, "assertion_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };

    let mut mgr = state.plan_manager.lock().await;
    match mgr.remove_assertion(&plan_id, &group_id, &request_id, &assertion_id) {
        Ok(()) => tool_ok(format!("Assertion {assertion_id} removed")),
        Err(e) => tool_error(e.to_string()),
    }
}

async fn handle_update_assertion(args: Value, state: &ToolState) -> ToolCallResult {
    let plan_id = match require_str(&args, "plan_id").and_then(|s| parse_uuid(s, "plan_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let group_id = match require_str(&args, "group_id").and_then(|s| parse_uuid(s, "group_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let request_id = match require_str(&args, "request_id").and_then(|s| parse_uuid(s, "request_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let assertion_id = match require_str(&args, "assertion_id").and_then(|s| parse_uuid(s, "assertion_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };

    let name = args.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
    let rule = args.get("rule").cloned();

    let mut mgr = state.plan_manager.lock().await;
    match mgr.update_assertion(&plan_id, &group_id, &request_id, &assertion_id, name, rule) {
        Ok(assertion) => json_ok(&assertion),
        Err(e) => tool_error(e.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Extractor handlers
// ---------------------------------------------------------------------------

async fn handle_add_extractor(args: Value, state: &ToolState) -> ToolCallResult {
    let plan_id = match require_str(&args, "plan_id").and_then(|s| parse_uuid(s, "plan_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let group_id = match require_str(&args, "group_id").and_then(|s| parse_uuid(s, "group_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let request_id = match require_str(&args, "request_id").and_then(|s| parse_uuid(s, "request_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let name = match require_str(&args, "name") {
        Ok(v) => v.to_string(),
        Err(e) => return tool_error(e),
    };
    let variable = match require_str(&args, "variable") {
        Ok(v) => v.to_string(),
        Err(e) => return tool_error(e),
    };
    let expression = match args.get("expression") {
        Some(v) => v.clone(),
        None => return tool_error("Missing required argument: expression".to_string()),
    };

    let mut mgr = state.plan_manager.lock().await;
    match mgr.add_extractor(&plan_id, &group_id, &request_id, name, variable, expression) {
        Ok(extractor) => json_ok(&extractor),
        Err(e) => tool_error(e.to_string()),
    }
}

async fn handle_remove_extractor(args: Value, state: &ToolState) -> ToolCallResult {
    let plan_id = match require_str(&args, "plan_id").and_then(|s| parse_uuid(s, "plan_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let group_id = match require_str(&args, "group_id").and_then(|s| parse_uuid(s, "group_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let request_id = match require_str(&args, "request_id").and_then(|s| parse_uuid(s, "request_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let extractor_id = match require_str(&args, "extractor_id").and_then(|s| parse_uuid(s, "extractor_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };

    let mut mgr = state.plan_manager.lock().await;
    match mgr.remove_extractor(&plan_id, &group_id, &request_id, &extractor_id) {
        Ok(()) => tool_ok(format!("Extractor {extractor_id} removed")),
        Err(e) => tool_error(e.to_string()),
    }
}

async fn handle_update_extractor(args: Value, state: &ToolState) -> ToolCallResult {
    let plan_id = match require_str(&args, "plan_id").and_then(|s| parse_uuid(s, "plan_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let group_id = match require_str(&args, "group_id").and_then(|s| parse_uuid(s, "group_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let request_id = match require_str(&args, "request_id").and_then(|s| parse_uuid(s, "request_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let extractor_id = match require_str(&args, "extractor_id").and_then(|s| parse_uuid(s, "extractor_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };

    let name = args.get("name").and_then(|v| v.as_str()).map(|s| s.to_string());
    let variable = args.get("variable").and_then(|v| v.as_str()).map(|s| s.to_string());
    let expression = args.get("expression").cloned();

    let mut mgr = state.plan_manager.lock().await;
    match mgr.update_extractor(
        &plan_id,
        &group_id,
        &request_id,
        &extractor_id,
        name,
        variable,
        expression,
    ) {
        Ok(extractor) => json_ok(&extractor),
        Err(e) => tool_error(e.to_string()),
    }
}

// ---------------------------------------------------------------------------
// Engine handlers
// ---------------------------------------------------------------------------

async fn handle_run_test(args: Value, state: &ToolState) -> ToolCallResult {
    let plan_id = match require_str(&args, "plan_id").and_then(|s| parse_uuid(s, "plan_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };

    let plan = {
        let mgr = state.plan_manager.lock().await;
        match mgr.get_plan(&plan_id).cloned() {
            Some(p) => p,
            None => return tool_error(format!("Plan not found: {plan_id}")),
        }
    };

    let (tx, mut rx) = tokio::sync::mpsc::channel(4096);
    let config = EngineConfig {
        plan,
        result_tx: tx,
    };

    let handle = match engine::run_test(config).await {
        Ok(h) => h,
        Err(e) => return tool_error(format!("Failed to start engine: {e}")),
    };

    {
        let mut h = state.engine_handle.lock().await;
        *h = Some(handle);
    }

    let mut summary = None;
    while let Some(event) = rx.recv().await {
        match &event {
            EngineEvent::Complete { summary: s } => {
                summary = Some(s.clone());
                break;
            }
            EngineEvent::Progress {
                completed_requests,
                current_rps,
                ..
            } => {
                eprintln!(
                    "[rmeter-mcp] Progress: {} requests, {:.1} rps",
                    completed_requests, current_rps
                );
            }
            EngineEvent::StatusChange { status } => {
                eprintln!("[rmeter-mcp] Status: {status}");
            }
            _ => {}
        }
    }

    {
        let mut h = state.engine_handle.lock().await;
        *h = None;
    }

    match summary {
        Some(s) => json_ok(&s),
        None => tool_error("Test completed but no summary was produced".to_string()),
    }
}

async fn handle_get_test_status(state: &ToolState) -> ToolCallResult {
    let handle = state.engine_handle.lock().await;
    let status = match &*handle {
        Some(h) => h.status.read().await.clone(),
        None => EngineStatus::Idle,
    };
    json_ok(&status)
}

async fn handle_stop_test(state: &ToolState) -> ToolCallResult {
    let handle = state.engine_handle.lock().await;
    match &*handle {
        Some(h) => {
            h.cancel_token.cancel();
            tool_ok("Stop signal sent to engine".to_string())
        }
        None => tool_error("No test is currently running".to_string()),
    }
}

async fn handle_get_results(state: &ToolState) -> ToolCallResult {
    let handle = state.engine_handle.lock().await;
    match &*handle {
        Some(h) => {
            let snapshot: AggregatorSnapshot = h.aggregator.read().await.snapshot();
            json_ok(&snapshot)
        }
        None => tool_error("No test is currently running".to_string()),
    }
}

// ---------------------------------------------------------------------------
// File I/O handlers
// ---------------------------------------------------------------------------

async fn handle_load_test_plan(args: Value, state: &ToolState) -> ToolCallResult {
    let path = match require_str(&args, "path") {
        Ok(v) => v.to_string(),
        Err(e) => return tool_error(e),
    };

    let plan = match plan_io::read_plan(&path).await {
        Ok(p) => p,
        Err(e) => return tool_error(format!("Failed to load plan from {path}: {e}")),
    };

    let result = json_ok(&plan);
    let mut mgr = state.plan_manager.lock().await;
    mgr.add_plan(plan);
    result
}

async fn handle_save_test_plan(args: Value, state: &ToolState) -> ToolCallResult {
    let plan_id = match require_str(&args, "plan_id").and_then(|s| parse_uuid(s, "plan_id")) {
        Ok(v) => v,
        Err(e) => return tool_error(e),
    };
    let path = match require_str(&args, "path") {
        Ok(v) => v.to_string(),
        Err(e) => return tool_error(e),
    };

    let plan = {
        let mgr = state.plan_manager.lock().await;
        match mgr.get_plan(&plan_id).cloned() {
            Some(p) => p,
            None => return tool_error(format!("Plan not found: {plan_id}")),
        }
    };

    match plan_io::write_plan(&plan, &path).await {
        Ok(()) => tool_ok(format!("Plan {plan_id} saved to {path}")),
        Err(e) => tool_error(format!("Failed to save plan to {path}: {e}")),
    }
}
