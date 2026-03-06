# rmeter vs JMeter — Feature Comparison

A detailed comparison of rmeter's current functionality against Apache JMeter 5.x.

---

## Overview

| Aspect | rmeter | JMeter |
|--------|--------|--------|
| **Language** | Rust (core) + React/TypeScript (UI) | Java |
| **Architecture** | Desktop app (Tauri v2) + MCP server | Desktop GUI (Swing) + CLI mode |
| **Platforms** | macOS, Linux, Windows | Any JVM platform |
| **License** | RLX Rixmerz License (RXL) v1.1 | Apache License 2.0 |
| **AI Integration** | Built-in MCP server (29 tools) | None (third-party plugins only) |

---

## 1. Protocols Supported

| Protocol | rmeter | JMeter |
|----------|--------|--------|
| HTTP/HTTPS | Yes | Yes |
| WebSocket (ws/wss) | Yes | Via plugin (JMeter WebSocket Samplers) |
| GraphQL | Yes (dedicated support with introspection) | Via HTTP sampler (manual setup) |
| FTP | No | Yes |
| JDBC (Database) | No | Yes |
| LDAP | No | Yes |
| JMS (Java Messaging) | No | Yes |
| SMTP/POP3/IMAP (Mail) | No | Yes |
| TCP | No | Yes |
| MongoDB | No | Via plugin |
| gRPC | No | Via plugin |
| AMQP/RabbitMQ | No | Via plugin |

**Summary:** rmeter focuses on web protocols (HTTP, WebSocket, GraphQL). JMeter covers a much broader set of protocols including databases, messaging, mail, and LDAP.

---

## 2. Test Plan Structure

| Feature | rmeter | JMeter |
|---------|--------|--------|
| Test Plans | Yes | Yes |
| Thread Groups | Yes (num_threads, ramp_up, loop_count) | Yes (+ scheduler, delays, startup config) |
| Multiple Thread Groups | Yes | Yes |
| Thread Group enable/disable | Yes | Yes |
| Ramp-up period | Yes (linear ramp, seconds) | Yes (linear ramp, seconds) |
| Loop Count — Finite | Yes | Yes |
| Loop Count — Duration-based | Yes (seconds) | Yes (scheduler with start/end time) |
| Loop Count — Infinite | Yes | Yes |
| Stepping Thread Group | No | Yes (via plugin) |
| Ultimate Thread Group | No | Yes (via plugin) |
| Concurrency Thread Group | No | Yes (via plugin) |
| setUp Thread Group | Yes | Yes |
| tearDown Thread Group | Yes | Yes |
| Test Fragment | No | Yes |

---

## 3. Samplers / Request Types

| Feature | rmeter | JMeter |
|---------|--------|--------|
| HTTP Request | Yes (GET, POST, PUT, DELETE, PATCH, HEAD, OPTIONS) | Yes (all methods) |
| Request Body — JSON | Yes | Yes |
| Request Body — Form Data | Yes (URL-encoded) | Yes (URL-encoded + multipart) |
| Request Body — Raw text | Yes | Yes |
| Request Body — XML | Yes | Yes |
| Request Body — File upload | No | Yes (multipart/form-data) |
| Custom headers | Yes | Yes |
| Cookies | Yes (via reqwest cookie jar) | Yes (Cookie Manager) |
| Redirects | Yes (automatic, via reqwest) | Yes (configurable) |
| Compression (gzip/brotli) | Yes (built-in) | Yes (gzip/deflate) |
| HTTP/2 | Via reqwest (automatic negotiation) | Via plugin |
| WebSocket Sampler | Yes (connect, send text/binary, receive, delay, close) | Via plugin |
| GraphQL Sampler | Yes (query, mutation, variables, operation name, introspection) | No (use HTTP sampler manually) |
| JDBC Request | No | Yes |
| FTP Request | No | Yes |
| LDAP Request | No | Yes |
| JMS Sampler | No | Yes |
| TCP Sampler | No | Yes |
| OS Process Sampler | No | Yes |
| Debug Sampler | No | Yes |
| JSR223 Sampler | No | Yes |

---

## 4. Authentication

| Feature | rmeter | JMeter |
|---------|--------|--------|
| Bearer Token | Yes | Yes (via Header Manager) |
| Basic Auth | Yes (username + optional password) | Yes (HTTP Authorization Manager) |
| Digest Auth | No | Yes |
| Kerberos/NTLM | No | Yes |
| OAuth 1.0/2.0 | No | Via plugin |
| Client Certificate (mTLS) | No | Yes |

---

## 5. Assertions / Validations

| Feature | rmeter | JMeter |
|---------|--------|--------|
| Status Code Equals | Yes | Yes |
| Status Code Not Equals | Yes | Yes (via negation) |
| Status Code Range | Yes (min-max inclusive) | Yes (via regex or BeanShell) |
| Body Contains | Yes (substring) | Yes |
| Body Not Contains | Yes | Yes |
| JSON Path assertion | Yes (dot-notation: `data.id`, `items[0].name`) | Yes (full JSONPath with `$.` syntax) |
| Response Time assertion | Yes (threshold in ms) | Yes (Duration Assertion) |
| Header Equals | Yes | Yes |
| Header Contains | Yes | Yes |
| Regex assertion | Yes (body_matches_regex) | Yes (Response Assertion with regex) |
| XML/XPath assertion | No | Yes |
| HTML assertion | No | Yes |
| Size assertion | No | Yes |
| MD5Hex assertion | No | Yes |
| JSR223 assertion | No | Yes |
| Compare assertion | No | Yes (via plugin) |

---

## 6. Extractors / Post-Processors

| Feature | rmeter | JMeter |
|---------|--------|--------|
| JSON Path extractor | Yes (dot-notation) | Yes (full JSONPath) |
| Regex extractor | Yes (capture groups) | Yes |
| Header extractor | Yes (by name) | Yes |
| XPath extractor | No | Yes |
| CSS/jQuery extractor | No | Yes |
| Boundary extractor | No | Yes |
| JSR223 Post-Processor | No | Yes |
| Debug Post-Processor | No | Yes |
| Result Status Action Handler | No | Yes |

---

## 7. Variables & Parameterization

| Feature | rmeter | JMeter |
|---------|--------|--------|
| User-defined variables | Yes (`${name}` syntax) | Yes |
| Variable scopes (Global, Plan, ThreadGroup) | Yes | Partially (function-based scoping) |
| CSV Data Set Config | Yes (with sharing mode & recycle) | Yes (more options: delimiter, EOF action, sharing mode) |
| CSV sharing — All Threads | Yes | Yes |
| CSV sharing — Per Thread | Yes | Yes |
| CSV sharing — Per Thread Group | No | Yes |
| Variable substitution in URLs | Yes | Yes |
| Variable substitution in headers | Yes | Yes |
| Variable substitution in body | Yes | Yes |
| Built-in functions (`__Random`, `__time`, etc.) | Yes (`__random`, `__randomString`, `__time`, `__uuid`, `__counter`, `__threadNum`, `__property`) | Yes (50+ built-in functions) |
| Counter element | Yes (`__counter()` function) | Yes |
| Random variable element | Yes (`__random()`, `__randomString()` functions) | Yes |
| User Parameters | No | Yes |

---

## 8. Timers

| Feature | rmeter | JMeter |
|---------|--------|--------|
| Constant Timer | Yes | Yes |
| Gaussian Random Timer | Yes | Yes |
| Uniform Random Timer | Yes | Yes |
| Poisson Random Timer | No | Yes |
| Synchronizing Timer | No | Yes |
| Constant Throughput Timer | No | Yes |
| Precise Throughput Timer | No | Yes |
| WebSocket Delay step | Yes (fixed ms delay in WS scenarios) | N/A |

**Note:** rmeter supports constant, gaussian random, and uniform random timers configured per thread group. Requests within a thread group execute sequentially with configurable think-time delays.

---

## 9. Logic Controllers

| Feature | rmeter | JMeter |
|---------|--------|--------|
| Simple Controller | No | Yes |
| Loop Controller | Yes (nested, configurable count) | Yes |
| If Controller | Yes (condition evaluation with `==`, `!=`, truthy) | Yes |
| While Controller | No | Yes |
| Switch Controller | No | Yes |
| ForEach Controller | No | Yes |
| Transaction Controller | Yes (aggregate timing for grouped requests) | Yes |
| Module Controller | No | Yes |
| Include Controller | No | Yes |
| Runtime Controller | No | Yes |
| Random Controller | No | Yes |
| Random Order Controller | No | Yes |
| Interleave Controller | No | Yes |
| Once Only Controller | No | Yes |
| Throughput Controller | No | Yes |

**Note:** rmeter supports nested test elements with If, Loop, and Transaction controllers. Elements can be arbitrarily nested for complex flow control.

---

## 10. Listeners / Reporting

| Feature | rmeter | JMeter |
|---------|--------|--------|
| Real-time dashboard | Yes (built-in Tauri GUI with live charts) | Yes (HTML dashboard report, Grafana via backend listener) |
| CSV export | Yes (with summary comments) | Yes |
| JSON export | Yes (full results) | No (via plugins) |
| HTML report | Yes (standalone, inline CSS, dark theme) | Yes (comprehensive dashboard) |
| Comparison report (HTML) | Yes (side-by-side delta between two runs) | No (manual comparison or plugin) |
| Result history / in-memory store | Yes (last N runs) | No (saved to disk only) |
| Time-series per-second data | Yes | Yes |
| Percentile metrics (p50, p95, p99) | Yes | Yes |
| Throughput (req/s) | Yes | Yes |
| Error rate tracking | Yes | Yes |
| Bytes received | Yes | Yes |
| Response body capture | Yes (truncated to 4 KB per request) | Yes (View Results Tree) |
| Request/response detail view | Yes (method, URL, headers, body) | Yes |
| Aggregate Report | Via summary statistics | Yes (dedicated listener) |
| View Results Tree | Via individual request results | Yes (dedicated listener) |
| Summary Report | Via TestSummary | Yes |
| Graph Results | Via real-time dashboard charts | Yes |
| Backend Listener (InfluxDB, Graphite) | No | Yes |
| Simple Data Writer | No | Yes |

---

## 11. Pre-Processors

| Feature | rmeter | JMeter |
|---------|--------|--------|
| Variable substitution (automatic) | Yes (in URL, headers, body) | Yes (via `${var}` syntax) |
| JSR223 Pre-Processor | No | Yes |
| HTML Link Parser | No | Yes |
| HTTP URL Rewriting Modifier | No | Yes |
| User Parameters | No | Yes |
| BeanShell Pre-Processor | No | Yes |

---

## 12. Configuration Elements

| Feature | rmeter | JMeter |
|---------|--------|--------|
| HTTP Request Defaults | Yes (base_url + shared headers) | Yes |
| HTTP Header Manager | Yes (via HTTP Defaults shared headers) | Yes (shared defaults) |
| HTTP Cookie Manager | Automatic (reqwest) | Yes (configurable policies) |
| HTTP Cache Manager | No | Yes |
| HTTP Authorization Manager | Auth per-request | Yes (centralized) |
| CSV Data Set Config | Yes | Yes |
| User Defined Variables | Yes | Yes |
| Keystore Configuration | No | Yes |
| JDBC Connection Config | No | Yes |
| DNS Cache Manager | No | Yes |

---

## 13. Test Templates

| Feature | rmeter | JMeter |
|---------|--------|--------|
| REST API Test | Yes (1 user, GET/POST/PUT) | No built-in (user creates) |
| Load Test | Yes (10 users, 10s ramp-up, 60s duration) | No built-in |
| Stress Test | Yes (100 users, 30s ramp-up, 120s duration) | No built-in |
| Recording Template | No | Yes |
| SOAP WebService Test | No | Yes (template) |

---

## 14. File I/O & Plan Management

| Feature | rmeter | JMeter |
|---------|--------|--------|
| Save/load plans | Yes (`.rmeter` JSON files) | Yes (`.jmx` XML files) |
| Plan validation | Yes (basic) | Yes |
| Plan Manager (multiple plans in memory) | Yes | No (one plan at a time) |
| Plan format versioning | Yes | Yes |
| Import/export | JSON-based | XML-based |

---

## 15. Engine & Execution

| Feature | rmeter | JMeter |
|---------|--------|--------|
| Async I/O model | Yes (Tokio async runtime) | Thread-per-user (Java threads) |
| Connection pooling | Yes (reqwest pool with configurable idle) | Yes (Apache HttpClient) |
| Graceful stop | Yes (CancellationToken) | Yes |
| Engine status tracking | Yes (Idle/Running/Stopping/Completed/Error) | Yes |
| Progress events (live) | Yes (every 500ms: RPS, mean, p95, active threads) | Yes (via listeners) |
| Request enable/disable | Yes | Yes |
| Custom timeouts | Yes (configurable per-client) | Yes |
| TLS/SSL | Yes (via native-tls) | Yes (via Java JSSE) |
| Accept invalid certificates | Yes (configurable) | Yes |
| Custom User-Agent | Yes | Yes |
| HTTP proxy support | Via reqwest (env vars) | Yes (built-in config) |

---

## 16. Distributed / Remote Testing

| Feature | rmeter | JMeter |
|---------|--------|--------|
| Distributed testing (remote agents) | No | Yes (JMeter Remote Testing) |
| Master-slave architecture | No | Yes |
| Cloud-based execution | No | Via BlazeMeter, OctoPerf, etc. |

---

## 17. AI & Automation Integration

| Feature | rmeter | JMeter |
|---------|--------|--------|
| MCP Server | Yes (29 tools via stdio JSON-RPC) | No |
| AI-driven test creation | Yes (Claude Desktop, Claude Code) | No |
| Programmatic plan manipulation | Yes (MCP tools: CRUD for plans, groups, requests, variables, assertions, extractors, CSV sources) | Via REST API (limited) or Taurus |
| AI-driven test execution & analysis | Yes (run_test, get_results, get_test_status, stop_test) | No |

---

## 18. Scripting & Extensibility

| Feature | rmeter | JMeter |
|---------|--------|--------|
| Built-in scripting language | No | Yes (Groovy/JSR223, BeanShell) |
| Plugin system | Not yet (planned) | Yes (extensive ecosystem) |
| Custom sampler development | Not yet | Yes (Java plugins) |
| JMeter Plugins Manager | N/A | Yes |
| Recording proxy | No | Yes (HTTP(S) Test Script Recorder) |
| CLI (non-GUI) mode | MCP server (headless) | Yes (`jmeter -n -t test.jmx`) |

---

## 19. HTTP Client Capabilities

| Feature | rmeter | JMeter |
|---------|--------|--------|
| HTTP/1.1 | Yes | Yes |
| HTTP/2 | Yes (via reqwest/hyper) | Via plugin |
| Connection keep-alive | Yes | Yes |
| Connection pool tuning | Yes (max idle per host, idle timeout) | Yes |
| gzip decompression | Yes | Yes |
| Brotli decompression | Yes | No (default) |
| Cookie persistence | Yes (automatic) | Yes (Cookie Manager) |
| Redirect following | Yes (automatic) | Yes (configurable) |

---

## Summary of rmeter Strengths vs JMeter

1. **Performance** — Rust + Tokio async I/O vs Java threads; lower memory footprint and higher throughput potential.
2. **Modern UI** — React/TypeScript desktop app with real-time dashboard vs Swing GUI.
3. **AI-native** — Built-in MCP server allowing AI assistants to create, configure, and run tests autonomously.
4. **WebSocket & GraphQL** — First-class support out of the box, including GraphQL introspection.
5. **Comparison reports** — Built-in A/B comparison between test runs with delta metrics.
6. **Modern formats** — JSON-based plan files and exports vs XML.
7. **HTTP/2 & Brotli** — Supported out of the box.

## Summary of JMeter Strengths vs rmeter

1. **Protocol breadth** — JDBC, LDAP, JMS, FTP, SMTP, TCP, and many more.
2. **Logic controllers** — Full set of while/switch/forEach/random/interleave/once-only controllers (rmeter covers if/loop/transaction).
3. **Timers** — Additional timer types (poisson, synchronizing, throughput) beyond rmeter's constant/gaussian/uniform.
4. **Scripting** — Groovy/JSR223 for custom logic anywhere in the test plan.
5. **Plugin ecosystem** — Hundreds of community plugins via JMeter Plugins Manager.
6. **Distributed testing** — Built-in remote agent architecture for scaling across machines.
7. **Recording proxy** — Capture browser traffic and auto-generate test plans.
8. **Built-in functions** — 50+ functions vs rmeter's 7 core functions.
9. **Pre/post processors** — Extensive hooks before and after each request.
10. **Configuration elements** — Caching and advanced cookie policies.
