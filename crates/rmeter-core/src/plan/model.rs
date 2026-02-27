use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

// ---------------------------------------------------------------------------
// HttpMethod
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Patch => "PATCH",
            HttpMethod::Head => "HEAD",
            HttpMethod::Options => "OPTIONS",
        };
        write!(f, "{s}")
    }
}

// ---------------------------------------------------------------------------
// RequestBody
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RequestBody {
    /// A JSON payload (stored as a raw JSON string for flexibility).
    Json(String),
    /// URL-encoded form data as ordered key/value pairs.
    FormData(Vec<(String, String)>),
    /// Arbitrary raw bytes/text body.
    Raw(String),
    /// An XML payload.
    Xml(String),
}

// ---------------------------------------------------------------------------
// LoopCount
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LoopCount {
    /// Run for a fixed number of iterations.
    Finite { count: u64 },
    /// Run for a fixed wall-clock duration in seconds.
    Duration { seconds: u64 },
    /// Run forever until explicitly stopped.
    Infinite,
}

impl Default for LoopCount {
    fn default() -> Self {
        Self::Finite { count: 1 }
    }
}

// ---------------------------------------------------------------------------
// Assertion / Extractor stubs
// ---------------------------------------------------------------------------

/// A single assertion applied to an HTTP response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Assertion {
    pub id: Uuid,
    pub name: String,
    /// JSON-encoded assertion rule — schema defined by the assertion engine.
    pub rule: serde_json::Value,
}

/// A data extractor that captures values from an HTTP response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Extractor {
    pub id: Uuid,
    pub name: String,
    /// Variable name where the extracted value is stored.
    pub variable: String,
    /// JSON-encoded extraction expression — schema defined by the extractor engine.
    pub expression: serde_json::Value,
}

// ---------------------------------------------------------------------------
// HttpRequest
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct HttpRequest {
    pub id: Uuid,
    pub name: String,
    pub method: HttpMethod,
    /// Target URL; may contain `{{variable}}` placeholders.
    pub url: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<RequestBody>,
    #[serde(default)]
    pub assertions: Vec<Assertion>,
    #[serde(default)]
    pub extractors: Vec<Extractor>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_true() -> bool {
    true
}

// ---------------------------------------------------------------------------
// WebSocket types
// ---------------------------------------------------------------------------

/// A single step in a WebSocket test scenario.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WebSocketStep {
    /// Connect to a WebSocket server (recorded as latency measurement).
    Connect { url: String, headers: HashMap<String, String> },
    /// Send a UTF-8 text frame.
    SendText { message: String },
    /// Send a binary frame; `data` is a base64-encoded byte string.
    SendBinary { data: String },
    /// Wait for the next message from the server (with timeout in ms).
    Receive { timeout_ms: u64 },
    /// Sleep for a fixed duration (ms) to simulate think time.
    Delay { duration_ms: u64 },
    /// Send a close frame and shut down the connection.
    Close,
}

/// A WebSocket test scenario containing an ordered sequence of steps.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct WebSocketRequest {
    pub id: Uuid,
    pub name: String,
    /// Initial connection URL (`ws://` or `wss://`).
    pub url: String,
    /// HTTP upgrade headers applied during the handshake.
    #[serde(default)]
    pub headers: HashMap<String, String>,
    /// Ordered list of steps to execute after connecting.
    pub steps: Vec<WebSocketStep>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

// ---------------------------------------------------------------------------
// Variable
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VariableScope {
    Global,
    #[default]
    Plan,
    ThreadGroup,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Variable {
    pub id: Uuid,
    pub name: String,
    pub value: String,
    #[serde(default)]
    pub scope: VariableScope,
}

// ---------------------------------------------------------------------------
// CSV Data Source
// ---------------------------------------------------------------------------

/// Controls how CSV rows are distributed across virtual users.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CsvSharingMode {
    /// All threads share a single row counter (round-robin across all users).
    #[default]
    AllThreads,
    /// Each thread maintains its own row counter independently.
    PerThread,
}

/// A CSV data source that feeds variable values into the test plan.
///
/// Similar to JMeter's CSV Data Set Config — each column name becomes a
/// variable that can be referenced as `${column_name}` in URLs, headers,
/// and request bodies. Each iteration reads the next row of data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CsvDataSource {
    pub id: Uuid,
    pub name: String,
    /// Column names parsed from the CSV header row.
    pub columns: Vec<String>,
    /// Data rows — each inner Vec has one value per column.
    pub rows: Vec<Vec<String>>,
    /// How rows are distributed across virtual users.
    #[serde(default)]
    pub sharing_mode: CsvSharingMode,
    /// Whether to wrap around to the first row when all rows are consumed.
    #[serde(default = "default_true")]
    pub recycle: bool,
}

// ---------------------------------------------------------------------------
// ThreadGroup
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ThreadGroup {
    pub id: Uuid,
    pub name: String,
    /// Number of concurrent virtual users.
    pub num_threads: u32,
    /// Time in seconds to ramp all threads up to `num_threads`.
    pub ramp_up_seconds: u32,
    #[serde(default)]
    pub loop_count: LoopCount,
    #[serde(default)]
    pub requests: Vec<HttpRequest>,
    #[serde(default = "default_true")]
    pub enabled: bool,
}

// ---------------------------------------------------------------------------
// TestPlan
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TestPlan {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub thread_groups: Vec<ThreadGroup>,
    #[serde(default)]
    pub variables: Vec<Variable>,
    #[serde(default)]
    pub csv_data_sources: Vec<CsvDataSource>,
    /// Format version for forward-compatibility.
    #[serde(default = "default_format_version")]
    pub format_version: u32,
}

fn default_format_version() -> u32 {
    1
}

impl TestPlan {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            description: String::new(),
            thread_groups: Vec::new(),
            variables: Vec::new(),
            csv_data_sources: Vec::new(),
            format_version: 1,
        }
    }
}

impl CsvDataSource {
    /// Parse CSV content (with header row) into a `CsvDataSource`.
    pub fn from_csv_content(name: impl Into<String>, content: &str, delimiter: u8) -> Result<Self, String> {
        let mut reader = csv::ReaderBuilder::new()
            .delimiter(delimiter)
            .has_headers(true)
            .flexible(true)
            .from_reader(content.as_bytes());

        let headers = reader
            .headers()
            .map_err(|e| format!("Failed to read CSV headers: {e}"))?
            .iter()
            .map(|h| h.trim().to_string())
            .collect::<Vec<_>>();

        if headers.is_empty() {
            return Err("CSV has no columns".to_string());
        }

        let mut rows = Vec::new();
        for result in reader.records() {
            let record = result.map_err(|e| format!("Failed to read CSV row: {e}"))?;
            let row: Vec<String> = record.iter().map(|f| f.to_string()).collect();
            rows.push(row);
        }

        if rows.is_empty() {
            return Err("CSV has no data rows".to_string());
        }

        Ok(Self {
            id: Uuid::new_v4(),
            name: name.into(),
            columns: headers,
            rows,
            sharing_mode: CsvSharingMode::default(),
            recycle: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // HttpMethod
    // -----------------------------------------------------------------------

    #[test]
    fn http_method_display() {
        assert_eq!(HttpMethod::Get.to_string(), "GET");
        assert_eq!(HttpMethod::Post.to_string(), "POST");
        assert_eq!(HttpMethod::Put.to_string(), "PUT");
        assert_eq!(HttpMethod::Delete.to_string(), "DELETE");
        assert_eq!(HttpMethod::Patch.to_string(), "PATCH");
        assert_eq!(HttpMethod::Head.to_string(), "HEAD");
        assert_eq!(HttpMethod::Options.to_string(), "OPTIONS");
    }

    #[test]
    fn http_method_serialize_screaming_snake_case() {
        let json = serde_json::to_string(&HttpMethod::Get).unwrap();
        assert_eq!(json, "\"GET\"");
        let json = serde_json::to_string(&HttpMethod::Post).unwrap();
        assert_eq!(json, "\"POST\"");
        let json = serde_json::to_string(&HttpMethod::Delete).unwrap();
        assert_eq!(json, "\"DELETE\"");
    }

    #[test]
    fn http_method_deserialize() {
        let method: HttpMethod = serde_json::from_str("\"GET\"").unwrap();
        assert_eq!(method, HttpMethod::Get);
        let method: HttpMethod = serde_json::from_str("\"POST\"").unwrap();
        assert_eq!(method, HttpMethod::Post);
        let method: HttpMethod = serde_json::from_str("\"PATCH\"").unwrap();
        assert_eq!(method, HttpMethod::Patch);
    }

    #[test]
    fn http_method_equality() {
        assert_eq!(HttpMethod::Get, HttpMethod::Get);
        assert_ne!(HttpMethod::Get, HttpMethod::Post);
    }

    // -----------------------------------------------------------------------
    // RequestBody
    // -----------------------------------------------------------------------

    #[test]
    fn request_body_json_construction_and_match() {
        let body = RequestBody::Json("{\"key\": \"value\"}".to_string());
        match body {
            RequestBody::Json(s) => assert_eq!(s, "{\"key\": \"value\"}"),
            _ => panic!("expected Json variant"),
        }
    }

    #[test]
    fn request_body_form_data_construction_and_match() {
        let body = RequestBody::FormData(vec![
            ("key1".to_string(), "val1".to_string()),
            ("key2".to_string(), "val2".to_string()),
        ]);
        match body {
            RequestBody::FormData(pairs) => {
                assert_eq!(pairs.len(), 2);
                assert_eq!(pairs[0], ("key1".to_string(), "val1".to_string()));
            }
            _ => panic!("expected FormData variant"),
        }
    }

    #[test]
    fn request_body_raw_construction_and_match() {
        let body = RequestBody::Raw("raw text body".to_string());
        match body {
            RequestBody::Raw(s) => assert_eq!(s, "raw text body"),
            _ => panic!("expected Raw variant"),
        }
    }

    #[test]
    fn request_body_xml_construction_and_match() {
        let body = RequestBody::Xml("<root/>".to_string());
        match body {
            RequestBody::Xml(s) => assert_eq!(s, "<root/>"),
            _ => panic!("expected Xml variant"),
        }
    }

    // -----------------------------------------------------------------------
    // LoopCount
    // -----------------------------------------------------------------------

    #[test]
    fn loop_count_default_is_finite_one() {
        let lc = LoopCount::default();
        match lc {
            LoopCount::Finite { count } => assert_eq!(count, 1),
            _ => panic!("expected Finite"),
        }
    }

    #[test]
    fn loop_count_finite_serde() {
        let lc = LoopCount::Finite { count: 42 };
        let json = serde_json::to_string(&lc).unwrap();
        let parsed: LoopCount = serde_json::from_str(&json).unwrap();
        match parsed {
            LoopCount::Finite { count } => assert_eq!(count, 42),
            _ => panic!("expected Finite"),
        }
    }

    #[test]
    fn loop_count_duration_serde() {
        let lc = LoopCount::Duration { seconds: 120 };
        let json = serde_json::to_string(&lc).unwrap();
        let parsed: LoopCount = serde_json::from_str(&json).unwrap();
        match parsed {
            LoopCount::Duration { seconds } => assert_eq!(seconds, 120),
            _ => panic!("expected Duration"),
        }
    }

    #[test]
    fn loop_count_infinite_serde() {
        let lc = LoopCount::Infinite;
        let json = serde_json::to_string(&lc).unwrap();
        let parsed: LoopCount = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed, LoopCount::Infinite));
    }

    // -----------------------------------------------------------------------
    // VariableScope
    // -----------------------------------------------------------------------

    #[test]
    fn variable_scope_default_is_plan() {
        let scope = VariableScope::default();
        assert!(matches!(scope, VariableScope::Plan));
    }

    #[test]
    fn variable_scope_serde_roundtrip() {
        for scope in [VariableScope::Global, VariableScope::Plan, VariableScope::ThreadGroup] {
            let json = serde_json::to_string(&scope).unwrap();
            let parsed: VariableScope = serde_json::from_str(&json).unwrap();
            // Use serialized form for comparison since VariableScope doesn't impl PartialEq
            assert_eq!(
                serde_json::to_string(&parsed).unwrap(),
                json
            );
        }
    }

    // -----------------------------------------------------------------------
    // TestPlan
    // -----------------------------------------------------------------------

    #[test]
    fn test_plan_new_has_defaults() {
        let plan = TestPlan::new("My Plan");
        assert_eq!(plan.name, "My Plan");
        assert_eq!(plan.description, "");
        assert!(plan.thread_groups.is_empty());
        assert!(plan.variables.is_empty());
        assert_eq!(plan.format_version, 1);
    }

    #[test]
    fn test_plan_new_generates_unique_ids() {
        let plan1 = TestPlan::new("Plan A");
        let plan2 = TestPlan::new("Plan B");
        assert_ne!(plan1.id, plan2.id);
    }

    #[test]
    fn test_plan_serde_roundtrip() {
        let mut plan = TestPlan::new("Serde Plan");
        plan.description = "A test plan for serialization".to_string();
        plan.variables.push(Variable {
            id: Uuid::new_v4(),
            name: "base_url".to_string(),
            value: "http://example.com".to_string(),
            scope: VariableScope::Global,
        });
        plan.thread_groups.push(ThreadGroup {
            id: Uuid::new_v4(),
            name: "TG1".to_string(),
            num_threads: 5,
            ramp_up_seconds: 10,
            loop_count: LoopCount::Duration { seconds: 60 },
            requests: vec![HttpRequest {
                id: Uuid::new_v4(),
                name: "GET /".to_string(),
                method: HttpMethod::Get,
                url: "http://example.com".to_string(),
                headers: HashMap::new(),
                body: None,
                assertions: Vec::new(),
                extractors: Vec::new(),
                enabled: true,
            }],
            enabled: true,
        });

        let json = serde_json::to_string_pretty(&plan).unwrap();
        let parsed: TestPlan = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, plan.id);
        assert_eq!(parsed.name, plan.name);
        assert_eq!(parsed.description, plan.description);
        assert_eq!(parsed.format_version, plan.format_version);
        assert_eq!(parsed.thread_groups.len(), 1);
        assert_eq!(parsed.variables.len(), 1);
        assert_eq!(parsed.thread_groups[0].num_threads, 5);
    }

    // -----------------------------------------------------------------------
    // HttpRequest
    // -----------------------------------------------------------------------

    #[test]
    fn http_request_enabled_default_is_true() {
        let json = r#"{
            "id": "00000000-0000-0000-0000-000000000001",
            "name": "Test",
            "method": "GET",
            "url": "http://example.com"
        }"#;
        let req: HttpRequest = serde_json::from_str(json).unwrap();
        assert!(req.enabled);
    }

    #[test]
    fn http_request_optional_body_defaults_to_none() {
        let json = r#"{
            "id": "00000000-0000-0000-0000-000000000001",
            "name": "Test",
            "method": "GET",
            "url": "http://example.com"
        }"#;
        let req: HttpRequest = serde_json::from_str(json).unwrap();
        assert!(req.body.is_none());
    }

    #[test]
    fn http_request_headers_default_empty() {
        let json = r#"{
            "id": "00000000-0000-0000-0000-000000000001",
            "name": "Test",
            "method": "GET",
            "url": "http://example.com"
        }"#;
        let req: HttpRequest = serde_json::from_str(json).unwrap();
        assert!(req.headers.is_empty());
    }

    // -----------------------------------------------------------------------
    // ThreadGroup
    // -----------------------------------------------------------------------

    #[test]
    fn thread_group_serde_roundtrip() {
        let tg = ThreadGroup {
            id: Uuid::new_v4(),
            name: "Workers".to_string(),
            num_threads: 50,
            ramp_up_seconds: 30,
            loop_count: LoopCount::Infinite,
            requests: Vec::new(),
            enabled: true,
        };
        let json = serde_json::to_string(&tg).unwrap();
        let parsed: ThreadGroup = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, tg.id);
        assert_eq!(parsed.name, tg.name);
        assert_eq!(parsed.num_threads, 50);
        assert_eq!(parsed.ramp_up_seconds, 30);
        assert!(matches!(parsed.loop_count, LoopCount::Infinite));
    }

    // -----------------------------------------------------------------------
    // WebSocketStep / WebSocketRequest
    // -----------------------------------------------------------------------

    #[test]
    fn websocket_step_serde_roundtrip() {
        let steps = vec![
            WebSocketStep::Connect {
                url: "ws://localhost:8080".to_string(),
                headers: HashMap::new(),
            },
            WebSocketStep::SendText {
                message: "hello".to_string(),
            },
            WebSocketStep::SendBinary {
                data: "AQID".to_string(),
            },
            WebSocketStep::Receive { timeout_ms: 5000 },
            WebSocketStep::Delay { duration_ms: 1000 },
            WebSocketStep::Close,
        ];
        for step in steps {
            let json = serde_json::to_string(&step).unwrap();
            let _parsed: WebSocketStep = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn websocket_request_serde_roundtrip() {
        let req = WebSocketRequest {
            id: Uuid::new_v4(),
            name: "WS Test".to_string(),
            url: "ws://localhost:8080".to_string(),
            headers: HashMap::new(),
            steps: vec![WebSocketStep::Close],
            enabled: true,
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: WebSocketRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, req.id);
        assert_eq!(parsed.name, "WS Test");
    }
}
