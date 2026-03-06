//! JMeter `.jmx` file parser — converts JMX XML into an rmeter [`TestPlan`].

use std::collections::HashMap;

use quick_xml::events::Event;
use quick_xml::Reader;
use uuid::Uuid;

use crate::plan::model::{
    Assertion, CsvDataSource, CsvSharingMode, Extractor, HttpMethod, HttpRequest, LoopCount,
    RequestBody, TestPlan, ThreadGroup, Variable, VariableScope,
};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse a JMeter `.jmx` XML string into an rmeter [`TestPlan`].
pub fn parse_jmx(xml: &str) -> Result<TestPlan, String> {
    let root = parse_xml_tree(xml)?;
    convert_jmx_tree(&root)
}

// ---------------------------------------------------------------------------
// Simple XML tree representation
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct XmlNode {
    tag: String,
    attrs: HashMap<String, String>,
    text: String,
    children: Vec<XmlNode>,
}

impl XmlNode {
    fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.get(name).map(|s| s.as_str())
    }

    fn child(&self, tag: &str) -> Option<&XmlNode> {
        self.children.iter().find(|c| c.tag == tag)
    }

    fn find_string_prop(&self, name: &str) -> Option<String> {
        self.children
            .iter()
            .find(|c| c.tag == "stringProp" && c.attr("name") == Some(name))
            .map(|c| c.text.clone())
    }

    fn find_bool_prop(&self, name: &str) -> Option<bool> {
        self.children
            .iter()
            .find(|c| c.tag == "boolProp" && c.attr("name") == Some(name))
            .map(|c| c.text.trim() == "true")
    }

    fn find_int_prop(&self, name: &str) -> Option<i64> {
        self.children
            .iter()
            .find(|c| c.tag == "intProp" && c.attr("name") == Some(name))
            .and_then(|c| c.text.trim().parse().ok())
    }
}

// ---------------------------------------------------------------------------
// XML tree parser (using quick-xml)
// ---------------------------------------------------------------------------

fn parse_xml_tree(xml: &str) -> Result<XmlNode, String> {
    let mut reader = Reader::from_str(xml);
    let mut root_children = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                let node = parse_element(&mut reader, e)?;
                root_children.push(node);
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(e) => return Err(format!("XML parse error: {e}")),
        }
    }

    Ok(XmlNode {
        tag: "__root__".to_string(),
        attrs: HashMap::new(),
        text: String::new(),
        children: root_children,
    })
}

fn parse_element(
    reader: &mut Reader<&[u8]>,
    start: &quick_xml::events::BytesStart<'_>,
) -> Result<XmlNode, String> {
    let tag = String::from_utf8_lossy(start.name().as_ref()).to_string();
    let mut attrs = HashMap::new();
    for attr in start.attributes().flatten() {
        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
        let val = String::from_utf8_lossy(&attr.value).to_string();
        attrs.insert(key, val);
    }

    let mut children = Vec::new();
    let mut text = String::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(ref e)) => {
                children.push(parse_element(reader, e)?);
            }
            Ok(Event::Empty(ref e)) => {
                let empty_tag = String::from_utf8_lossy(e.name().as_ref()).to_string();
                let mut empty_attrs = HashMap::new();
                for attr in e.attributes().flatten() {
                    let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                    let val = String::from_utf8_lossy(&attr.value).to_string();
                    empty_attrs.insert(key, val);
                }
                children.push(XmlNode {
                    tag: empty_tag,
                    attrs: empty_attrs,
                    text: String::new(),
                    children: Vec::new(),
                });
            }
            Ok(Event::Text(ref e)) => {
                text.push_str(&e.unescape().unwrap_or_default());
            }
            Ok(Event::CData(ref e)) => {
                text.push_str(&String::from_utf8_lossy(e.as_ref()));
            }
            Ok(Event::End(_)) => break,
            Ok(Event::Eof) => return Err(format!("Unexpected EOF inside <{tag}>")),
            Ok(_) => {}
            Err(e) => return Err(format!("XML parse error inside <{tag}>: {e}")),
        }
    }

    Ok(XmlNode {
        tag,
        attrs,
        text,
        children,
    })
}

// ---------------------------------------------------------------------------
// JMX tree → rmeter TestPlan conversion
// ---------------------------------------------------------------------------

fn convert_jmx_tree(root: &XmlNode) -> Result<TestPlan, String> {
    // Find the <jmeterTestPlan> → <hashTree> → <TestPlan> structure
    let jmeter_plan = root
        .child("jmeterTestPlan")
        .ok_or("Missing <jmeterTestPlan> root element")?;

    let outer_hash = jmeter_plan
        .child("hashTree")
        .ok_or("Missing outer <hashTree>")?;

    // The TestPlan element
    let test_plan_node = outer_hash
        .children
        .iter()
        .find(|c| c.tag == "TestPlan")
        .ok_or("Missing <TestPlan> element")?;

    let plan_name = test_plan_node
        .attr("testname")
        .unwrap_or("Imported JMX Plan")
        .to_string();

    // The inner hashTree contains all the plan-level children
    let inner_hash = outer_hash
        .children
        .iter()
        .filter(|c| c.tag == "hashTree")
        .last()
        .ok_or("Missing inner <hashTree>")?;

    let mut variables = Vec::new();
    let mut csv_data_sources = Vec::new();
    let mut thread_groups = Vec::new();

    // Process children in pairs: element + hashTree
    let children = &inner_hash.children;
    let mut i = 0;
    while i < children.len() {
        let node = &children[i];
        // Find the corresponding hashTree (next sibling)
        let hash_tree = if i + 1 < children.len() && children[i + 1].tag == "hashTree" {
            Some(&children[i + 1])
        } else {
            None
        };

        match node.tag.as_str() {
            "Arguments" => {
                let enabled = node.attr("enabled").unwrap_or("true") == "true";
                if enabled {
                    variables.extend(parse_arguments(node));
                }
                if hash_tree.is_some() {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "CSVDataSet" => {
                let enabled = node.attr("enabled").unwrap_or("true") == "true";
                if enabled {
                    if let Some(csv) = parse_csv_data_set(node) {
                        csv_data_sources.push(csv);
                    }
                }
                if hash_tree.is_some() {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "ThreadGroup" | "SetupThreadGroup" | "PostThreadGroup" => {
                if let Some(ht) = hash_tree {
                    // Collect plan-level headers (HeaderManager) that appear before this ThreadGroup
                    let tg = parse_thread_group(node, ht, &variables);
                    thread_groups.push(tg);
                    i += 2;
                } else {
                    i += 1;
                }
            }
            _ => {
                // Skip unknown elements; advance past their hashTree if present
                if hash_tree.is_some() {
                    i += 2;
                } else {
                    i += 1;
                }
            }
        }
    }

    Ok(TestPlan {
        id: Uuid::new_v4(),
        name: plan_name,
        description: String::new(),
        thread_groups,
        variables,
        csv_data_sources,
        format_version: 1,
    })
}

// ---------------------------------------------------------------------------
// Arguments → Variables
// ---------------------------------------------------------------------------

fn parse_arguments(node: &XmlNode) -> Vec<Variable> {
    let mut vars = Vec::new();

    // Arguments → collectionProp → elementProp*
    let collection = find_collection_prop(node, "Arguments.arguments");
    if let Some(col) = collection {
        for elem in &col.children {
            if elem.tag == "elementProp" {
                let name = elem
                    .find_string_prop("Argument.name")
                    .unwrap_or_default();
                let value = elem
                    .find_string_prop("Argument.value")
                    .unwrap_or_default();
                if !name.is_empty() {
                    vars.push(Variable {
                        id: Uuid::new_v4(),
                        name,
                        value,
                        scope: VariableScope::Plan,
                    });
                }
            }
        }
    }

    vars
}

// ---------------------------------------------------------------------------
// CSVDataSet → CsvDataSource
// ---------------------------------------------------------------------------

fn parse_csv_data_set(node: &XmlNode) -> Option<CsvDataSource> {
    let name = node.attr("testname").unwrap_or("CSV Data").to_string();
    let filename = node.find_string_prop("filename").unwrap_or_default();
    let variable_names = node.find_string_prop("variableNames").unwrap_or_default();
    let delimiter = node.find_string_prop("delimiter").unwrap_or_else(|| ",".to_string());
    let recycle = node.find_bool_prop("recycle").unwrap_or(true);
    let share_mode = node
        .find_string_prop("shareMode")
        .unwrap_or_else(|| "shareMode.all".to_string());

    let columns: Vec<String> = variable_names
        .split(&delimiter)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if columns.is_empty() {
        return None;
    }

    let sharing_mode = if share_mode.contains("thread") {
        CsvSharingMode::PerThread
    } else {
        CsvSharingMode::AllThreads
    };

    // We can't read the actual CSV file content, so we create a source with
    // the column definitions and a note about the original filename.
    // The rows will be empty — the user will need to load the CSV separately.
    Some(CsvDataSource {
        id: Uuid::new_v4(),
        name: format!("{name} ({filename})"),
        columns,
        rows: Vec::new(),
        sharing_mode,
        recycle,
    })
}

// ---------------------------------------------------------------------------
// ThreadGroup
// ---------------------------------------------------------------------------

fn parse_thread_group(
    node: &XmlNode,
    hash_tree: &XmlNode,
    _plan_variables: &[Variable],
) -> ThreadGroup {
    let name = node
        .attr("testname")
        .unwrap_or("Thread Group")
        .to_string();

    let num_threads = node
        .find_int_prop("ThreadGroup.num_threads")
        .or_else(|| {
            node.find_string_prop("ThreadGroup.num_threads")
                .and_then(|s| s.parse().ok())
        })
        .unwrap_or(1) as u32;

    let ramp_up = node
        .find_int_prop("ThreadGroup.ramp_time")
        .or_else(|| {
            node.find_string_prop("ThreadGroup.ramp_time")
                .and_then(|s| s.parse().ok())
        })
        .unwrap_or(1) as u32;

    // Loop count from the nested LoopController
    let loop_count = node
        .children
        .iter()
        .find(|c| c.tag == "elementProp" && c.attr("elementType") == Some("LoopController"))
        .and_then(|lc| {
            let loops_str = lc.find_string_prop("LoopController.loops")?;
            if loops_str == "-1" {
                Some(LoopCount::Infinite)
            } else {
                let count: u64 = loops_str.parse().ok()?;
                Some(LoopCount::Finite { count })
            }
        })
        .unwrap_or_default();

    let enabled = node.attr("enabled").unwrap_or("true") == "true";

    // Parse requests and their children from the hashTree
    let requests = parse_requests_from_hash_tree(hash_tree);

    ThreadGroup {
        id: Uuid::new_v4(),
        name,
        num_threads,
        ramp_up_seconds: ramp_up,
        loop_count,
        requests,
        enabled,
    }
}

// ---------------------------------------------------------------------------
// Requests parsing (recursive through hashTrees)
// ---------------------------------------------------------------------------

fn parse_requests_from_hash_tree(hash_tree: &XmlNode) -> Vec<HttpRequest> {
    let mut requests = Vec::new();
    // Collect any thread-group-level HeaderManager headers
    let shared_headers = collect_header_managers(hash_tree);

    let children = &hash_tree.children;
    let mut i = 0;
    while i < children.len() {
        let node = &children[i];
        let child_hash = if i + 1 < children.len() && children[i + 1].tag == "hashTree" {
            Some(&children[i + 1])
        } else {
            None
        };

        match node.tag.as_str() {
            "HTTPSamplerProxy" => {
                let enabled = node.attr("enabled").unwrap_or("true") == "true";
                let mut req = parse_http_sampler(node, &shared_headers);
                req.enabled = enabled;

                // Parse assertions and extractors from the child hashTree
                if let Some(ht) = child_hash {
                    parse_request_children(ht, &mut req);
                }

                requests.push(req);
                if child_hash.is_some() {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            "GenericController" | "TransactionController" | "IfController"
            | "WhileController" | "LoopController" | "ForeachController" => {
                // Logic controllers: recurse into their hashTree
                if let Some(ht) = child_hash {
                    requests.extend(parse_requests_from_hash_tree(ht));
                    i += 2;
                } else {
                    i += 1;
                }
            }
            _ => {
                if child_hash.is_some() {
                    i += 2;
                } else {
                    i += 1;
                }
            }
        }
    }

    requests
}

/// Collect headers from all HeaderManager nodes that are direct children.
fn collect_header_managers(hash_tree: &XmlNode) -> HashMap<String, String> {
    let mut headers = HashMap::new();
    for node in &hash_tree.children {
        if node.tag == "HeaderManager" {
            let enabled = node.attr("enabled").unwrap_or("true") == "true";
            if enabled {
                if let Some(col) = find_collection_prop(node, "HeaderManager.headers") {
                    for elem in &col.children {
                        if elem.tag == "elementProp" {
                            let name = elem.find_string_prop("Header.name").unwrap_or_default();
                            let value = elem.find_string_prop("Header.value").unwrap_or_default();
                            if !name.is_empty() {
                                headers.insert(name, value);
                            }
                        }
                    }
                }
            }
        }
    }
    headers
}

// ---------------------------------------------------------------------------
// HTTPSamplerProxy → HttpRequest
// ---------------------------------------------------------------------------

fn parse_http_sampler(node: &XmlNode, shared_headers: &HashMap<String, String>) -> HttpRequest {
    let name = node
        .attr("testname")
        .unwrap_or("HTTP Request")
        .to_string();

    let protocol = node
        .find_string_prop("HTTPSampler.protocol")
        .unwrap_or_else(|| "https".to_string());
    let domain = node
        .find_string_prop("HTTPSampler.domain")
        .unwrap_or_default();
    let port = node.find_string_prop("HTTPSampler.port").unwrap_or_default();
    let path = node
        .find_string_prop("HTTPSampler.path")
        .unwrap_or_default();

    // Build URL — preserve ${variable} references
    let url = build_url(&protocol, &domain, &port, &path);

    let method_str = node
        .find_string_prop("HTTPSampler.method")
        .unwrap_or_else(|| "GET".to_string());
    let method = match method_str.to_uppercase().as_str() {
        "POST" => HttpMethod::Post,
        "PUT" => HttpMethod::Put,
        "DELETE" => HttpMethod::Delete,
        "PATCH" => HttpMethod::Patch,
        "HEAD" => HttpMethod::Head,
        "OPTIONS" => HttpMethod::Options,
        _ => HttpMethod::Get,
    };

    // Parse body
    let post_body_raw = node.find_bool_prop("HTTPSampler.postBodyRaw").unwrap_or(false);
    let body = if post_body_raw {
        // Raw body from the Arguments element
        extract_raw_body(node)
    } else {
        // Form-encoded parameters
        extract_form_body(node)
    };

    // Headers: start with shared, can be overridden later by request-level HeaderManager
    let headers = shared_headers.clone();

    HttpRequest {
        id: Uuid::new_v4(),
        name,
        method,
        url,
        headers,
        body,
        assertions: Vec::new(),
        extractors: Vec::new(),
        enabled: true,
    }
}

fn build_url(protocol: &str, domain: &str, port: &str, path: &str) -> String {
    let mut url = String::new();

    if !protocol.is_empty() {
        url.push_str(protocol);
        url.push_str("://");
    } else {
        url.push_str("https://");
    }

    url.push_str(domain);

    if !port.is_empty() && port != "443" && port != "80" {
        url.push(':');
        url.push_str(port);
    } else if !port.is_empty() {
        // Keep variable references even for standard ports
        if port.contains("${") {
            url.push(':');
            url.push_str(port);
        }
    }

    if !path.is_empty() {
        if !path.starts_with('/') {
            url.push('/');
        }
        url.push_str(path);
    }

    url
}

fn extract_raw_body(node: &XmlNode) -> Option<RequestBody> {
    // Look for elementProp with HTTPsampler.Arguments → collectionProp → elementProp → Argument.value
    let args_prop = node
        .children
        .iter()
        .find(|c| {
            c.tag == "elementProp"
                && (c.attr("name") == Some("HTTPsampler.Arguments")
                    || c.attr("name") == Some("HTTPSampler.Arguments"))
        })?;

    let collection = args_prop
        .children
        .iter()
        .find(|c| c.tag == "collectionProp")?;

    let first_arg = collection.children.first()?;
    let raw_value = first_arg.find_string_prop("Argument.value")?;

    if raw_value.trim().is_empty() {
        return None;
    }

    // Clean up &#xd; (carriage return entities that JMeter uses)
    let cleaned = raw_value.replace('\r', "");

    // Try to detect if it's JSON
    let trimmed = cleaned.trim();
    if (trimmed.starts_with('{') && trimmed.ends_with('}'))
        || (trimmed.starts_with('[') && trimmed.ends_with(']'))
    {
        Some(RequestBody::Raw { raw: cleaned })
    } else if trimmed.starts_with('<') {
        Some(RequestBody::Xml { xml: cleaned })
    } else {
        Some(RequestBody::Raw { raw: cleaned })
    }
}

fn extract_form_body(node: &XmlNode) -> Option<RequestBody> {
    let args_prop = node.children.iter().find(|c| {
        c.tag == "elementProp"
            && (c.attr("name") == Some("HTTPsampler.Arguments")
                || c.attr("name") == Some("HTTPSampler.Arguments"))
    })?;

    let collection = args_prop
        .children
        .iter()
        .find(|c| c.tag == "collectionProp")?;

    let mut pairs = Vec::new();
    for elem in &collection.children {
        if elem.tag == "elementProp" {
            let name = elem.find_string_prop("Argument.name").unwrap_or_default();
            let value = elem.find_string_prop("Argument.value").unwrap_or_default();
            if !name.is_empty() {
                pairs.push((name, value));
            }
        }
    }

    if pairs.is_empty() {
        None
    } else {
        Some(RequestBody::FormData { form_data: pairs })
    }
}

// ---------------------------------------------------------------------------
// Request children: assertions, extractors, header managers
// ---------------------------------------------------------------------------

fn parse_request_children(hash_tree: &XmlNode, request: &mut HttpRequest) {
    let children = &hash_tree.children;
    let mut i = 0;
    while i < children.len() {
        let node = &children[i];

        match node.tag.as_str() {
            "HeaderManager" => {
                // Request-level headers override shared headers
                let enabled = node.attr("enabled").unwrap_or("true") == "true";
                if enabled {
                    if let Some(col) = find_collection_prop(node, "HeaderManager.headers") {
                        for elem in &col.children {
                            if elem.tag == "elementProp" {
                                let name =
                                    elem.find_string_prop("Header.name").unwrap_or_default();
                                let value =
                                    elem.find_string_prop("Header.value").unwrap_or_default();
                                if !name.is_empty() {
                                    request.headers.insert(name, value);
                                }
                            }
                        }
                    }
                }
            }
            "JSONPathAssertion" => {
                let enabled = node.attr("enabled").unwrap_or("true") == "true";
                if enabled {
                    if let Some(assertion) = parse_json_path_assertion(node) {
                        request.assertions.push(assertion);
                    }
                }
            }
            "ResponseAssertion" => {
                let enabled = node.attr("enabled").unwrap_or("true") == "true";
                if enabled {
                    if let Some(assertion) = parse_response_assertion(node) {
                        request.assertions.push(assertion);
                    }
                }
            }
            "JSONPostProcessor" => {
                let enabled = node.attr("enabled").unwrap_or("true") == "true";
                if enabled {
                    if let Some(extractor) = parse_json_post_processor(node) {
                        request.extractors.push(extractor);
                    }
                }
            }
            "RegexExtractor" => {
                let enabled = node.attr("enabled").unwrap_or("true") == "true";
                if enabled {
                    if let Some(extractor) = parse_regex_extractor(node) {
                        request.extractors.push(extractor);
                    }
                }
            }
            "JSR223Assertion" | "BeanShellAssertion" => {
                let enabled = node.attr("enabled").unwrap_or("true") == "true";
                if enabled {
                    let assertion_name = node.attr("testname").unwrap_or("Script Assertion").to_string();
                    let script = node.find_string_prop("script").unwrap_or_default();
                    let lang = node.find_string_prop("scriptLanguage").unwrap_or_else(|| "groovy".to_string());
                    request.assertions.push(Assertion {
                        id: Uuid::new_v4(),
                        name: assertion_name,
                        rule: serde_json::json!({
                            "type": "script",
                            "language": lang,
                            "script": script,
                            "note": "Imported from JMX — script assertions are not directly executable in rmeter"
                        }),
                    });
                }
            }
            _ => {}
        }

        // Skip corresponding hashTree
        if i + 1 < children.len() && children[i + 1].tag == "hashTree" {
            i += 2;
        } else {
            i += 1;
        }
    }
}

// ---------------------------------------------------------------------------
// JSONPathAssertion
// ---------------------------------------------------------------------------

fn parse_json_path_assertion(node: &XmlNode) -> Option<Assertion> {
    let name = node
        .attr("testname")
        .unwrap_or("JSON Path Assertion")
        .to_string();
    let json_path = node.find_string_prop("JSON_PATH")?;
    let expected = node
        .find_string_prop("EXPECTED_VALUE")
        .unwrap_or_default();
    let invert = node.find_bool_prop("INVERT").unwrap_or(false);

    // Strip the leading "$." from JMeter JSON paths to match rmeter's
    // dot-notation navigator (e.g. "$.errors" → "errors",
    // "$.data.items[0].id" → "data.items[0].id").
    let expression = if json_path.starts_with("$.") {
        json_path[2..].to_string()
    } else if json_path == "$" {
        String::new()
    } else {
        json_path.clone()
    };

    // Map to the appropriate rmeter assertion rule:
    //  - invert + empty expected → JsonPathNotExists (path must NOT exist)
    //  - !invert + empty expected → JsonPathExists (path must exist)
    //  - !invert + non-empty expected → JsonPath (path must equal expected)
    let rule = if invert && expected.is_empty() {
        serde_json::json!({
            "type": "json_path_not_exists",
            "expression": expression,
        })
    } else if !invert && expected.is_empty() {
        serde_json::json!({
            "type": "json_path_exists",
            "expression": expression,
        })
    } else if invert {
        // invert + expected value → assert path does NOT equal value
        // rmeter doesn't have a "not equals" variant, so approximate with
        // BodyNotContains of the expected value.
        serde_json::json!({
            "type": "body_not_contains",
            "substring": expected,
        })
    } else {
        // !invert + expected value → JsonPath equals check
        // Try to parse the expected value as JSON; fall back to string.
        let expected_value = serde_json::from_str::<serde_json::Value>(&expected)
            .unwrap_or_else(|_| serde_json::Value::String(expected.clone()));
        serde_json::json!({
            "type": "json_path",
            "expression": expression,
            "expected": expected_value,
        })
    };

    Some(Assertion {
        id: Uuid::new_v4(),
        name,
        rule,
    })
}

// ---------------------------------------------------------------------------
// ResponseAssertion
// ---------------------------------------------------------------------------

fn parse_response_assertion(node: &XmlNode) -> Option<Assertion> {
    let name = node
        .attr("testname")
        .unwrap_or("Response Assertion")
        .to_string();
    let test_field = node
        .find_string_prop("Assertion.test_field")
        .unwrap_or_else(|| "Assertion.response_data".to_string());

    // Collect test strings
    let mut test_strings = Vec::new();
    if let Some(col) = find_collection_prop(node, "Asserion.test_strings") {
        for child in &col.children {
            if child.tag == "stringProp" {
                test_strings.push(child.text.clone());
            }
        }
    }
    // Also try the correct spelling
    if test_strings.is_empty() {
        if let Some(col) = find_collection_prop(node, "Assertion.test_strings") {
            for child in &col.children {
                if child.tag == "stringProp" {
                    test_strings.push(child.text.clone());
                }
            }
        }
    }

    // Map to rmeter assertion rules. JMeter response assertions check that
    // the response body contains (or not) certain strings.
    // test_type bitmask: 2 = contains, 1 = matches, 8 = equals, 16 = substring
    // bit 2 of test_type = NOT
    let test_type: i32 = node
        .find_string_prop("Assertion.test_type")
        .and_then(|s| s.parse().ok())
        .unwrap_or(2); // default: contains

    let is_not = (test_type & 4) != 0;

    // For the response body field, map each test string to a BodyContains or BodyNotContains.
    // We use the first test string since rmeter assertions are 1:1.
    if test_field.contains("response_data") || test_field.contains("response_body") {
        let substring = test_strings.into_iter().next().unwrap_or_default();
        let rule = if is_not {
            serde_json::json!({
                "type": "body_not_contains",
                "substring": substring,
            })
        } else {
            serde_json::json!({
                "type": "body_contains",
                "substring": substring,
            })
        };
        Some(Assertion {
            id: Uuid::new_v4(),
            name,
            rule,
        })
    } else if test_field.contains("response_code") {
        // Status code assertion
        let code_str = test_strings.into_iter().next().unwrap_or_default();
        if let Ok(code) = code_str.parse::<u16>() {
            let rule = if is_not {
                serde_json::json!({
                    "type": "status_code_not_equals",
                    "not_expected": code,
                })
            } else {
                serde_json::json!({
                    "type": "status_code_equals",
                    "expected": code,
                })
            };
            Some(Assertion {
                id: Uuid::new_v4(),
                name,
                rule,
            })
        } else {
            None
        }
    } else {
        // Fallback: body contains
        let substring = test_strings.into_iter().next().unwrap_or_default();
        Some(Assertion {
            id: Uuid::new_v4(),
            name,
            rule: serde_json::json!({
                "type": "body_contains",
                "substring": substring,
            }),
        })
    }
}

// ---------------------------------------------------------------------------
// JSONPostProcessor → Extractor
// ---------------------------------------------------------------------------

fn parse_json_post_processor(node: &XmlNode) -> Option<Extractor> {
    let name = node
        .attr("testname")
        .unwrap_or("JSON Extractor")
        .to_string();
    let variable = node
        .find_string_prop("JSONPostProcessor.referenceNames")
        .unwrap_or_else(|| "extracted_var".to_string());
    let json_path = node
        .find_string_prop("JSONPostProcessor.jsonPathExprs")
        .unwrap_or_default();

    // Strip leading "$." from JMeter JSON paths for rmeter's dot-notation
    // navigator (e.g. "$.data.items[0].id" → "data.items[0].id").
    let expression = {
        let trimmed = json_path.trim();
        if trimmed.starts_with("$.") {
            trimmed[2..].to_string()
        } else if trimmed == "$" {
            String::new()
        } else {
            trimmed.to_string()
        }
    };

    Some(Extractor {
        id: Uuid::new_v4(),
        name,
        variable,
        expression: serde_json::json!({
            "type": "json_path",
            "expression": expression,
        }),
    })
}

// ---------------------------------------------------------------------------
// RegexExtractor → Extractor
// ---------------------------------------------------------------------------

fn parse_regex_extractor(node: &XmlNode) -> Option<Extractor> {
    let name = node
        .attr("testname")
        .unwrap_or("Regex Extractor")
        .to_string();
    let variable = node
        .find_string_prop("RegexExtractor.refname")
        .unwrap_or_else(|| "extracted_var".to_string());
    let regex = node
        .find_string_prop("RegexExtractor.regex")
        .unwrap_or_default();
    let template = node
        .find_string_prop("RegexExtractor.template")
        .unwrap_or_else(|| "$1$".to_string());

    // JMeter template "$1$" means capture group 1.
    // Parse the group number from the template pattern "$N$".
    let group: u32 = template
        .trim_start_matches('$')
        .trim_end_matches('$')
        .parse()
        .unwrap_or(1);

    Some(Extractor {
        id: Uuid::new_v4(),
        name,
        variable,
        expression: serde_json::json!({
            "type": "regex",
            "pattern": regex,
            "group": group,
        }),
    })
}

// ---------------------------------------------------------------------------
// Utility: find a collectionProp by name
// ---------------------------------------------------------------------------

fn find_collection_prop<'a>(node: &'a XmlNode, name: &str) -> Option<&'a XmlNode> {
    // Direct child
    if let Some(col) = node
        .children
        .iter()
        .find(|c| c.tag == "collectionProp" && c.attr("name") == Some(name))
    {
        return Some(col);
    }

    // Nested inside an elementProp or other container
    for child in &node.children {
        if let Some(col) = child
            .children
            .iter()
            .find(|c| c.tag == "collectionProp" && c.attr("name") == Some(name))
        {
            return Some(col);
        }
    }

    None
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_jmx() {
        let jmx = r#"<?xml version="1.0" encoding="UTF-8"?>
<jmeterTestPlan version="1.2" properties="5.0" jmeter="5.6.3">
  <hashTree>
    <TestPlan guiclass="TestPlanGui" testclass="TestPlan" testname="My Test Plan">
      <boolProp name="TestPlan.functional_mode">false</boolProp>
    </TestPlan>
    <hashTree>
      <ThreadGroup guiclass="ThreadGroupGui" testclass="ThreadGroup" testname="Users">
        <intProp name="ThreadGroup.num_threads">10</intProp>
        <intProp name="ThreadGroup.ramp_time">5</intProp>
        <elementProp name="ThreadGroup.main_controller" elementType="LoopController">
          <stringProp name="LoopController.loops">3</stringProp>
        </elementProp>
      </ThreadGroup>
      <hashTree>
        <HTTPSamplerProxy guiclass="HttpTestSampleGui" testclass="HTTPSamplerProxy" testname="GET Home" enabled="true">
          <stringProp name="HTTPSampler.domain">example.com</stringProp>
          <stringProp name="HTTPSampler.port">443</stringProp>
          <stringProp name="HTTPSampler.protocol">https</stringProp>
          <stringProp name="HTTPSampler.path">/api/home</stringProp>
          <stringProp name="HTTPSampler.method">GET</stringProp>
        </HTTPSamplerProxy>
        <hashTree/>
      </hashTree>
    </hashTree>
  </hashTree>
</jmeterTestPlan>"#;

        let plan = parse_jmx(jmx).unwrap();
        assert_eq!(plan.name, "My Test Plan");
        assert_eq!(plan.thread_groups.len(), 1);

        let tg = &plan.thread_groups[0];
        assert_eq!(tg.name, "Users");
        assert_eq!(tg.num_threads, 10);
        assert_eq!(tg.ramp_up_seconds, 5);
        assert!(matches!(tg.loop_count, LoopCount::Finite { count: 3 }));
        assert_eq!(tg.requests.len(), 1);

        let req = &tg.requests[0];
        assert_eq!(req.name, "GET Home");
        assert_eq!(req.url, "https://example.com/api/home");
        assert!(matches!(req.method, HttpMethod::Get));
    }

    #[test]
    fn parse_jmx_with_variables_and_csv() {
        let jmx = r#"<?xml version="1.0" encoding="UTF-8"?>
<jmeterTestPlan version="1.2" properties="5.0" jmeter="5.6.3">
  <hashTree>
    <TestPlan guiclass="TestPlanGui" testclass="TestPlan" testname="Var Test">
      <boolProp name="TestPlan.functional_mode">false</boolProp>
    </TestPlan>
    <hashTree>
      <Arguments guiclass="ArgumentsPanel" testclass="Arguments" testname="QA Variables" enabled="true">
        <collectionProp name="Arguments.arguments">
          <elementProp name="host" elementType="Argument">
            <stringProp name="Argument.name">host</stringProp>
            <stringProp name="Argument.value">api.example.com</stringProp>
            <stringProp name="Argument.metadata">=</stringProp>
          </elementProp>
          <elementProp name="port" elementType="Argument">
            <stringProp name="Argument.name">port</stringProp>
            <stringProp name="Argument.value">443</stringProp>
            <stringProp name="Argument.metadata">=</stringProp>
          </elementProp>
        </collectionProp>
      </Arguments>
      <hashTree/>
      <Arguments guiclass="ArgumentsPanel" testclass="Arguments" testname="Disabled Vars" enabled="false">
        <collectionProp name="Arguments.arguments">
          <elementProp name="host" elementType="Argument">
            <stringProp name="Argument.name">host</stringProp>
            <stringProp name="Argument.value">localhost</stringProp>
            <stringProp name="Argument.metadata">=</stringProp>
          </elementProp>
        </collectionProp>
      </Arguments>
      <hashTree/>
      <CSVDataSet guiclass="TestBeanGUI" testclass="CSVDataSet" testname="Dates" enabled="true">
        <stringProp name="filename">./data/dates.csv</stringProp>
        <stringProp name="variableNames">start_date,end_date</stringProp>
        <stringProp name="delimiter">,</stringProp>
        <boolProp name="recycle">true</boolProp>
        <stringProp name="shareMode">shareMode.all</stringProp>
      </CSVDataSet>
      <hashTree/>
      <ThreadGroup guiclass="ThreadGroupGui" testclass="ThreadGroup" testname="TG1">
        <intProp name="ThreadGroup.num_threads">1</intProp>
        <intProp name="ThreadGroup.ramp_time">1</intProp>
        <elementProp name="ThreadGroup.main_controller" elementType="LoopController">
          <stringProp name="LoopController.loops">1</stringProp>
        </elementProp>
      </ThreadGroup>
      <hashTree/>
    </hashTree>
  </hashTree>
</jmeterTestPlan>"#;

        let plan = parse_jmx(jmx).unwrap();
        assert_eq!(plan.name, "Var Test");

        // Only enabled variables should be imported
        assert_eq!(plan.variables.len(), 2);
        assert_eq!(plan.variables[0].name, "host");
        assert_eq!(plan.variables[0].value, "api.example.com");
        assert_eq!(plan.variables[1].name, "port");
        assert_eq!(plan.variables[1].value, "443");

        // CSV data source
        assert_eq!(plan.csv_data_sources.len(), 1);
        assert!(plan.csv_data_sources[0].name.contains("Dates"));
        assert_eq!(plan.csv_data_sources[0].columns, vec!["start_date", "end_date"]);
        assert!(plan.csv_data_sources[0].recycle);
    }

    #[test]
    fn parse_jmx_with_post_body_and_extractors() {
        let jmx = r#"<?xml version="1.0" encoding="UTF-8"?>
<jmeterTestPlan version="1.2" properties="5.0" jmeter="5.6.3">
  <hashTree>
    <TestPlan guiclass="TestPlanGui" testclass="TestPlan" testname="POST Test">
      <boolProp name="TestPlan.functional_mode">false</boolProp>
    </TestPlan>
    <hashTree>
      <ThreadGroup guiclass="ThreadGroupGui" testclass="ThreadGroup" testname="TG">
        <intProp name="ThreadGroup.num_threads">1</intProp>
        <intProp name="ThreadGroup.ramp_time">1</intProp>
        <elementProp name="ThreadGroup.main_controller" elementType="LoopController">
          <stringProp name="LoopController.loops">1</stringProp>
        </elementProp>
      </ThreadGroup>
      <hashTree>
        <HeaderManager guiclass="HeaderPanel" testclass="HeaderManager" testname="Headers">
          <collectionProp name="HeaderManager.headers">
            <elementProp name="" elementType="Header">
              <stringProp name="Header.name">Content-Type</stringProp>
              <stringProp name="Header.value">application/json</stringProp>
            </elementProp>
          </collectionProp>
        </HeaderManager>
        <hashTree/>
        <HTTPSamplerProxy guiclass="HttpTestSampleGui" testclass="HTTPSamplerProxy" testname="Search" enabled="true">
          <stringProp name="HTTPSampler.domain">api.example.com</stringProp>
          <stringProp name="HTTPSampler.port">443</stringProp>
          <stringProp name="HTTPSampler.protocol">https</stringProp>
          <stringProp name="HTTPSampler.path">/graphql</stringProp>
          <stringProp name="HTTPSampler.method">POST</stringProp>
          <boolProp name="HTTPSampler.postBodyRaw">true</boolProp>
          <elementProp name="HTTPsampler.Arguments" elementType="Arguments">
            <collectionProp name="Arguments.arguments">
              <elementProp name="" elementType="HTTPArgument">
                <boolProp name="HTTPArgument.always_encode">false</boolProp>
                <stringProp name="Argument.value">{"query": "{ hello }"}</stringProp>
              </elementProp>
            </collectionProp>
          </elementProp>
        </HTTPSamplerProxy>
        <hashTree>
          <JSONPathAssertion guiclass="JSONPathAssertionGui" testclass="JSONPathAssertion" testname="Check errors" enabled="true">
            <stringProp name="JSON_PATH">$.errors</stringProp>
            <stringProp name="EXPECTED_VALUE"></stringProp>
            <boolProp name="INVERT">true</boolProp>
            <boolProp name="ISREGEX">true</boolProp>
          </JSONPathAssertion>
          <hashTree/>
          <JSONPostProcessor guiclass="JSONPostProcessorGui" testclass="JSONPostProcessor" testname="Extract id" enabled="true">
            <stringProp name="JSONPostProcessor.referenceNames">itemId</stringProp>
            <stringProp name="JSONPostProcessor.jsonPathExprs">$.data.items[0].id</stringProp>
            <stringProp name="JSONPostProcessor.match_numbers"></stringProp>
          </JSONPostProcessor>
          <hashTree/>
        </hashTree>
      </hashTree>
    </hashTree>
  </hashTree>
</jmeterTestPlan>"#;

        let plan = parse_jmx(jmx).unwrap();
        let tg = &plan.thread_groups[0];
        assert_eq!(tg.requests.len(), 1);

        let req = &tg.requests[0];
        assert_eq!(req.name, "Search");
        assert!(matches!(req.method, HttpMethod::Post));
        assert_eq!(req.url, "https://api.example.com/graphql");

        // Headers from HeaderManager
        assert_eq!(req.headers.get("Content-Type").unwrap(), "application/json");

        // Body
        assert!(req.body.is_some());

        // Assertions
        assert_eq!(req.assertions.len(), 1);
        assert_eq!(req.assertions[0].name, "Check errors");

        // Extractors
        assert_eq!(req.extractors.len(), 1);
        assert_eq!(req.extractors[0].name, "Extract id");
        assert_eq!(req.extractors[0].variable, "itemId");
    }

    #[test]
    fn parse_jmx_with_logic_controller() {
        let jmx = r#"<?xml version="1.0" encoding="UTF-8"?>
<jmeterTestPlan version="1.2" properties="5.0" jmeter="5.6.3">
  <hashTree>
    <TestPlan guiclass="TestPlanGui" testclass="TestPlan" testname="Logic Test">
      <boolProp name="TestPlan.functional_mode">false</boolProp>
    </TestPlan>
    <hashTree>
      <ThreadGroup guiclass="ThreadGroupGui" testclass="ThreadGroup" testname="TG">
        <intProp name="ThreadGroup.num_threads">1</intProp>
        <intProp name="ThreadGroup.ramp_time">1</intProp>
        <elementProp name="ThreadGroup.main_controller" elementType="LoopController">
          <stringProp name="LoopController.loops">1</stringProp>
        </elementProp>
      </ThreadGroup>
      <hashTree>
        <HTTPSamplerProxy guiclass="HttpTestSampleGui" testclass="HTTPSamplerProxy" testname="Req 1" enabled="true">
          <stringProp name="HTTPSampler.domain">example.com</stringProp>
          <stringProp name="HTTPSampler.protocol">https</stringProp>
          <stringProp name="HTTPSampler.path">/1</stringProp>
          <stringProp name="HTTPSampler.method">GET</stringProp>
        </HTTPSamplerProxy>
        <hashTree/>
        <GenericController guiclass="LogicControllerGui" testclass="GenericController" testname="My Group"/>
        <hashTree>
          <HTTPSamplerProxy guiclass="HttpTestSampleGui" testclass="HTTPSamplerProxy" testname="Req 2" enabled="true">
            <stringProp name="HTTPSampler.domain">example.com</stringProp>
            <stringProp name="HTTPSampler.protocol">https</stringProp>
            <stringProp name="HTTPSampler.path">/2</stringProp>
            <stringProp name="HTTPSampler.method">POST</stringProp>
          </HTTPSamplerProxy>
          <hashTree/>
        </hashTree>
      </hashTree>
    </hashTree>
  </hashTree>
</jmeterTestPlan>"#;

        let plan = parse_jmx(jmx).unwrap();
        let tg = &plan.thread_groups[0];
        // Both requests should be flattened
        assert_eq!(tg.requests.len(), 2);
        assert_eq!(tg.requests[0].name, "Req 1");
        assert_eq!(tg.requests[1].name, "Req 2");
    }

    #[test]
    fn parse_full_sample_jmx() {
        let jmx = include_str!("../../tests/fixtures/sample.jmx");
        let plan = parse_jmx(jmx).unwrap();
        assert_eq!(plan.name, "B2B Manager Cart Test");
        // Should have enabled QA variables
        assert!(!plan.variables.is_empty());
        // Should have CSV data sources
        assert_eq!(plan.csv_data_sources.len(), 2);
        // Should have one thread group
        assert_eq!(plan.thread_groups.len(), 1);
        // Should have multiple requests
        assert!(plan.thread_groups[0].requests.len() > 3);
    }
}
