use chrono::SecondsFormat;

use super::{ComparisonResult, TestRunResult};

// ---------------------------------------------------------------------------
// CSV export
// ---------------------------------------------------------------------------

/// Export a test run as CSV.
///
/// Produces a text document with:
/// - Leading comment lines (prefixed `#`) containing the run summary.
/// - A header row.
/// - One data row per `RequestResultEvent`.
pub fn export_csv(result: &TestRunResult) -> String {
    let s = &result.summary;
    let error_rate = if s.total_requests > 0 {
        s.failed_requests as f64 / s.total_requests as f64 * 100.0
    } else {
        0.0
    };

    let duration_secs = (s.finished_at - s.started_at)
        .num_milliseconds()
        .max(0) as f64
        / 1000.0;

    let mut out = String::new();

    // Summary header comments.
    out.push_str(&format!("# rmeter test run — {}\n", s.plan_name));
    out.push_str(&format!(
        "# Run ID: {}\n",
        result.run_id.hyphenated()
    ));
    out.push_str(&format!(
        "# Started:  {}\n",
        s.started_at.to_rfc3339_opts(SecondsFormat::Millis, true)
    ));
    out.push_str(&format!(
        "# Finished: {}\n",
        s.finished_at.to_rfc3339_opts(SecondsFormat::Millis, true)
    ));
    out.push_str(&format!("# Duration: {:.3}s\n", duration_secs));
    out.push_str(&format!(
        "# Total requests: {}\n",
        s.total_requests
    ));
    out.push_str(&format!(
        "# Successful: {}\n",
        s.successful_requests
    ));
    out.push_str(&format!(
        "# Failed: {} ({:.2}%)\n",
        s.failed_requests, error_rate
    ));
    out.push_str(&format!(
        "# Throughput: {:.2} req/s\n",
        s.requests_per_second
    ));
    out.push_str(&format!(
        "# Mean response: {:.2}ms\n",
        s.mean_response_ms
    ));
    out.push_str(&format!(
        "# P50: {}ms  P95: {}ms  P99: {}ms\n",
        s.p50_response_ms, s.p95_response_ms, s.p99_response_ms
    ));
    out.push_str(&format!(
        "# Min: {}ms  Max: {}ms\n",
        s.min_response_ms, s.max_response_ms
    ));
    out.push('\n');

    // Column header.
    out.push_str(
        "timestamp,request_name,thread_group,status_code,elapsed_ms,size_bytes,success,error\n",
    );

    // Data rows.
    for r in &result.request_results {
        let ts = r.timestamp.to_rfc3339_opts(SecondsFormat::Millis, true);
        let request_name = csv_escape(&r.request_name);
        let thread_group = csv_escape(&r.thread_group_name);
        let success = if r.error.is_none() && r.assertions_passed {
            "true"
        } else {
            "false"
        };
        let error = r
            .error
            .as_deref()
            .map(csv_escape)
            .unwrap_or_default();

        out.push_str(&format!(
            "{},{},{},{},{},{},{},{}\n",
            ts,
            request_name,
            thread_group,
            r.status_code,
            r.elapsed_ms,
            r.size_bytes,
            success,
            error
        ));
    }

    out
}

/// Wrap a field value in quotes and escape any embedded quotes.
fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

// ---------------------------------------------------------------------------
// JSON export
// ---------------------------------------------------------------------------

/// Export a test run as pretty-printed JSON.
pub fn export_json(result: &TestRunResult) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(result)
}

// ---------------------------------------------------------------------------
// HTML export
// ---------------------------------------------------------------------------

/// Export a test run as a standalone HTML report with inline CSS.
///
/// No external dependencies — the returned string can be saved as a `.html`
/// file and opened directly in a browser.
pub fn export_html(result: &TestRunResult) -> String {
    let s = &result.summary;
    let error_rate = if s.total_requests > 0 {
        s.failed_requests as f64 / s.total_requests as f64 * 100.0
    } else {
        0.0
    };
    let success_rate = 100.0 - error_rate;

    let duration_secs = (s.finished_at - s.started_at)
        .num_milliseconds()
        .max(0) as f64
        / 1000.0;

    let started = s
        .started_at
        .to_rfc3339_opts(SecondsFormat::Millis, true);
    let finished = s
        .finished_at
        .to_rfc3339_opts(SecondsFormat::Millis, true);

    // --- time-series table rows ---
    let ts_rows: String = result
        .time_series
        .iter()
        .map(|entry| {
            let err_rate = if entry.requests > 0 {
                entry.errors as f64 / entry.requests as f64 * 100.0
            } else {
                0.0
            };
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{:.2}%</td>\
                 <td>{:.2}</td><td>{}</td><td>{}</td></tr>",
                entry.second,
                entry.requests,
                entry.errors,
                err_rate,
                entry.avg_ms,
                entry.min_ms,
                entry.max_ms,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    // --- request results table rows (first 500 rows shown) ---
    let result_rows: String = result
        .request_results
        .iter()
        .take(500)
        .map(|r| {
            let ts = r.timestamp.to_rfc3339_opts(SecondsFormat::Millis, true);
            let ok = r.error.is_none() && r.assertions_passed;
            let row_class = if ok { "ok" } else { "err" };
            let status_text = if r.status_code == 0 {
                "—".to_string()
            } else {
                r.status_code.to_string()
            };
            let error_text = r.error.as_deref().unwrap_or("").replace('<', "&lt;");
            format!(
                "<tr class=\"{}\"><td>{}</td><td>{}</td><td>{}</td>\
                 <td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                row_class,
                html_escape(&ts),
                html_escape(&r.request_name),
                html_escape(&r.thread_group_name),
                status_text,
                r.elapsed_ms,
                r.size_bytes,
                error_text,
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let result_count = result.request_results.len();
    let result_caption = if result_count > 500 {
        format!(
            "Showing first 500 of {} individual requests",
            result_count
        )
    } else {
        format!("{} individual requests", result_count)
    };

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>rmeter Report — {plan_name}</title>
<style>
  *, *::before, *::after {{ box-sizing: border-box; }}
  body {{
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    margin: 0; padding: 2rem;
    background: #0f172a; color: #e2e8f0;
    line-height: 1.5;
  }}
  h1 {{ font-size: 1.75rem; font-weight: 700; color: #f1f5f9; margin: 0 0 0.25rem; }}
  h2 {{ font-size: 1.125rem; font-weight: 600; color: #94a3b8;
        text-transform: uppercase; letter-spacing: 0.05em;
        margin: 2rem 0 0.75rem; border-bottom: 1px solid #1e293b; padding-bottom: 0.5rem; }}
  .meta {{ color: #64748b; font-size: 0.875rem; margin-bottom: 2rem; }}
  .meta span {{ margin-right: 1.5rem; }}
  .stats-grid {{
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
    gap: 1rem; margin-bottom: 2rem;
  }}
  .stat-card {{
    background: #1e293b; border: 1px solid #334155;
    border-radius: 0.5rem; padding: 1rem 1.25rem;
  }}
  .stat-card .label {{
    font-size: 0.75rem; text-transform: uppercase; letter-spacing: 0.05em;
    color: #64748b; margin-bottom: 0.25rem;
  }}
  .stat-card .value {{
    font-size: 1.5rem; font-weight: 700; color: #f1f5f9;
  }}
  .stat-card .unit {{ font-size: 0.875rem; color: #94a3b8; margin-left: 0.2rem; }}
  .stat-card.good .value {{ color: #34d399; }}
  .stat-card.warn .value {{ color: #fbbf24; }}
  .stat-card.bad  .value {{ color: #f87171; }}
  table {{
    width: 100%; border-collapse: collapse; font-size: 0.8125rem;
    background: #1e293b; border-radius: 0.5rem; overflow: hidden;
    margin-bottom: 2rem;
  }}
  thead {{ background: #0f172a; }}
  th {{
    padding: 0.625rem 0.875rem; text-align: left;
    font-weight: 600; color: #94a3b8;
    text-transform: uppercase; letter-spacing: 0.04em;
    font-size: 0.75rem;
  }}
  td {{ padding: 0.5rem 0.875rem; border-top: 1px solid #334155; color: #cbd5e1; }}
  tr.ok td {{ border-left: 3px solid #34d399; }}
  tr.err td {{ border-left: 3px solid #f87171; color: #fca5a5; }}
  tr:hover td {{ background: #243352; }}
  caption {{
    text-align: left; padding: 0.5rem 0; color: #64748b;
    font-size: 0.8125rem; caption-side: bottom;
  }}
  .run-id {{ font-family: monospace; font-size: 0.8rem; color: #475569; }}
  footer {{
    margin-top: 3rem; padding-top: 1rem; border-top: 1px solid #1e293b;
    color: #475569; font-size: 0.8125rem;
  }}
</style>
</head>
<body>
<h1>{plan_name}</h1>
<div class="meta">
  <span>Started: {started}</span>
  <span>Finished: {finished}</span>
  <span>Duration: {duration:.3}s</span>
  <span class="run-id">Run ID: {run_id}</span>
</div>

<h2>Summary</h2>
<div class="stats-grid">
  <div class="stat-card">
    <div class="label">Total Requests</div>
    <div class="value">{total_requests}</div>
  </div>
  <div class="stat-card {success_class}">
    <div class="label">Success Rate</div>
    <div class="value">{success_rate:.2}<span class="unit">%</span></div>
  </div>
  <div class="stat-card {error_class}">
    <div class="label">Error Rate</div>
    <div class="value">{error_rate:.2}<span class="unit">%</span></div>
  </div>
  <div class="stat-card">
    <div class="label">Throughput</div>
    <div class="value">{rps:.2}<span class="unit">req/s</span></div>
  </div>
  <div class="stat-card">
    <div class="label">Mean Response</div>
    <div class="value">{mean:.2}<span class="unit">ms</span></div>
  </div>
  <div class="stat-card">
    <div class="label">P50</div>
    <div class="value">{p50}<span class="unit">ms</span></div>
  </div>
  <div class="stat-card">
    <div class="label">P95</div>
    <div class="value">{p95}<span class="unit">ms</span></div>
  </div>
  <div class="stat-card">
    <div class="label">P99</div>
    <div class="value">{p99}<span class="unit">ms</span></div>
  </div>
  <div class="stat-card">
    <div class="label">Min</div>
    <div class="value">{min}<span class="unit">ms</span></div>
  </div>
  <div class="stat-card">
    <div class="label">Max</div>
    <div class="value">{max}<span class="unit">ms</span></div>
  </div>
  <div class="stat-card">
    <div class="label">Data Received</div>
    <div class="value">{bytes_mb:.2}<span class="unit">MB</span></div>
  </div>
</div>

<h2>Time Series (per second)</h2>
<table>
  <thead>
    <tr>
      <th>Second</th><th>Requests</th><th>Errors</th><th>Error Rate</th>
      <th>Avg (ms)</th><th>Min (ms)</th><th>Max (ms)</th>
    </tr>
  </thead>
  <tbody>
{ts_rows}
  </tbody>
</table>

<h2>Individual Requests</h2>
<table>
  <caption>{result_caption}</caption>
  <thead>
    <tr>
      <th>Timestamp</th><th>Request</th><th>Thread Group</th>
      <th>Status</th><th>Elapsed (ms)</th><th>Size (B)</th><th>Error</th>
    </tr>
  </thead>
  <tbody>
{result_rows}
  </tbody>
</table>

<footer>Generated by rmeter &bull; {finished}</footer>
</body>
</html>
"#,
        plan_name = html_escape(&s.plan_name),
        run_id = result.run_id.hyphenated(),
        started = started,
        finished = finished,
        duration = duration_secs,
        total_requests = s.total_requests,
        success_rate = success_rate,
        success_class = if success_rate >= 99.0 { "good" } else if success_rate >= 95.0 { "warn" } else { "bad" },
        error_rate = error_rate,
        error_class = if error_rate < 1.0 { "good" } else if error_rate < 5.0 { "warn" } else { "bad" },
        rps = s.requests_per_second,
        mean = s.mean_response_ms,
        p50 = s.p50_response_ms,
        p95 = s.p95_response_ms,
        p99 = s.p99_response_ms,
        min = s.min_response_ms,
        max = s.max_response_ms,
        bytes_mb = s.total_bytes_received as f64 / 1_048_576.0,
        ts_rows = ts_rows,
        result_caption = result_caption,
        result_rows = result_rows,
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

// ---------------------------------------------------------------------------
// Comparison HTML export
// ---------------------------------------------------------------------------

/// Export a [`ComparisonResult`] as a standalone HTML report.
pub fn export_comparison_html(cmp: &ComparisonResult) -> String {
    let fmt_delta_ms = |d: i64| -> String {
        if d > 0 {
            format!("+{}ms", d)
        } else if d < 0 {
            format!("{}ms", d)
        } else {
            "±0ms".to_string()
        }
    };

    let fmt_delta_f64 = |d: f64, unit: &str| -> String {
        if d > 0.01 {
            format!("+{:.2}{}", d, unit)
        } else if d < -0.01 {
            format!("{:.2}{}", d, unit)
        } else {
            format!("±0{}", unit)
        }
    };

    let fmt_delta_i64 = |d: i64| -> String {
        if d > 0 {
            format!("+{}", d)
        } else if d < 0 {
            format!("{}", d)
        } else {
            "±0".to_string()
        }
    };

    let a = &cmp.run_a;
    let b = &cmp.run_b;

    let error_rate_a = if a.total_requests > 0 {
        a.failed_requests as f64 / a.total_requests as f64 * 100.0
    } else {
        0.0
    };
    let error_rate_b = if b.total_requests > 0 {
        b.failed_requests as f64 / b.total_requests as f64 * 100.0
    } else {
        0.0
    };

    let delta_err_rate_pct = cmp.delta_error_rate * 100.0;

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>rmeter Comparison Report</title>
<style>
  *, *::before, *::after {{ box-sizing: border-box; }}
  body {{
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
    margin: 0; padding: 2rem;
    background: #0f172a; color: #e2e8f0; line-height: 1.5;
  }}
  h1 {{ font-size: 1.75rem; font-weight: 700; color: #f1f5f9; margin: 0 0 0.5rem; }}
  h2 {{ font-size: 1.125rem; font-weight: 600; color: #94a3b8;
        text-transform: uppercase; letter-spacing: 0.05em;
        margin: 2rem 0 0.75rem; border-bottom: 1px solid #1e293b; padding-bottom: 0.5rem; }}
  table {{
    width: 100%; border-collapse: collapse; font-size: 0.875rem;
    background: #1e293b; border-radius: 0.5rem; overflow: hidden;
    margin-bottom: 2rem;
  }}
  thead {{ background: #0f172a; }}
  th, td {{ padding: 0.625rem 1rem; text-align: left; border-top: 1px solid #334155; }}
  th {{ font-weight: 600; color: #94a3b8; text-transform: uppercase; font-size: 0.75rem; }}
  td {{ color: #cbd5e1; }}
  td.metric {{ color: #94a3b8; font-size: 0.8rem; text-transform: uppercase; letter-spacing: 0.04em; }}
  .pos {{ color: #f87171; }}
  .neg {{ color: #34d399; }}
  .neutral {{ color: #94a3b8; }}
  footer {{
    margin-top: 3rem; padding-top: 1rem; border-top: 1px solid #1e293b;
    color: #475569; font-size: 0.8125rem;
  }}
</style>
</head>
<body>
<h1>Comparison Report</h1>
<p style="color:#64748b">Run A: <strong style="color:#e2e8f0">{plan_a}</strong> &nbsp; vs &nbsp; Run B: <strong style="color:#e2e8f0">{plan_b}</strong></p>

<h2>Metrics</h2>
<table>
  <thead>
    <tr><th>Metric</th><th>Run A</th><th>Run B</th><th>Delta (B − A)</th></tr>
  </thead>
  <tbody>
    <tr>
      <td class="metric">Total Requests</td>
      <td>{total_a}</td><td>{total_b}</td>
      <td class="{req_class}">{delta_req}</td>
    </tr>
    <tr>
      <td class="metric">Mean Response (ms)</td>
      <td>{mean_a:.2}</td><td>{mean_b:.2}</td>
      <td class="{mean_class}">{delta_mean}</td>
    </tr>
    <tr>
      <td class="metric">P95 Response (ms)</td>
      <td>{p95_a}</td><td>{p95_b}</td>
      <td class="{p95_class}">{delta_p95}</td>
    </tr>
    <tr>
      <td class="metric">P99 Response (ms)</td>
      <td>{p99_a}</td><td>{p99_b}</td>
      <td class="{p99_class}">{delta_p99}</td>
    </tr>
    <tr>
      <td class="metric">Throughput (req/s)</td>
      <td>{rps_a:.2}</td><td>{rps_b:.2}</td>
      <td class="{rps_class}">{delta_rps}</td>
    </tr>
    <tr>
      <td class="metric">Error Rate</td>
      <td>{err_a:.2}%</td><td>{err_b:.2}%</td>
      <td class="{err_class}">{delta_err}</td>
    </tr>
  </tbody>
</table>

<footer>Generated by rmeter</footer>
</body>
</html>
"#,
        plan_a = html_escape(&a.plan_name),
        plan_b = html_escape(&b.plan_name),
        total_a = a.total_requests,
        total_b = b.total_requests,
        delta_req = fmt_delta_i64(cmp.delta_total_requests),
        req_class = delta_class_neutral(cmp.delta_total_requests as f64),
        mean_a = a.mean_response_ms,
        mean_b = b.mean_response_ms,
        delta_mean = fmt_delta_f64(cmp.delta_mean_ms, "ms"),
        mean_class = delta_class_lower_better(cmp.delta_mean_ms),
        p95_a = a.p95_response_ms,
        p95_b = b.p95_response_ms,
        delta_p95 = fmt_delta_ms(cmp.delta_p95_ms),
        p95_class = delta_class_lower_better(cmp.delta_p95_ms as f64),
        p99_a = a.p99_response_ms,
        p99_b = b.p99_response_ms,
        delta_p99 = fmt_delta_ms(cmp.delta_p99_ms),
        p99_class = delta_class_lower_better(cmp.delta_p99_ms as f64),
        rps_a = a.requests_per_second,
        rps_b = b.requests_per_second,
        delta_rps = fmt_delta_f64(cmp.delta_rps, " req/s"),
        rps_class = delta_class_higher_better(cmp.delta_rps),
        err_a = error_rate_a,
        err_b = error_rate_b,
        delta_err = fmt_delta_f64(delta_err_rate_pct, "%"),
        err_class = delta_class_lower_better(cmp.delta_error_rate),
    )
}

fn delta_class_lower_better(d: f64) -> &'static str {
    if d > 0.001 { "pos" } else if d < -0.001 { "neg" } else { "neutral" }
}

fn delta_class_higher_better(d: f64) -> &'static str {
    if d > 0.001 { "neg" } else if d < -0.001 { "pos" } else { "neutral" }
}

fn delta_class_neutral(_d: f64) -> &'static str {
    "neutral"
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // csv_escape
    // -----------------------------------------------------------------------

    #[test]
    fn csv_escape_plain_string() {
        assert_eq!(csv_escape("hello"), "hello");
    }

    #[test]
    fn csv_escape_string_with_comma() {
        assert_eq!(csv_escape("a,b"), "\"a,b\"");
    }

    #[test]
    fn csv_escape_string_with_quotes() {
        assert_eq!(csv_escape("say \"hi\""), "\"say \"\"hi\"\"\"");
    }

    #[test]
    fn csv_escape_string_with_newline() {
        assert_eq!(csv_escape("line1\nline2"), "\"line1\nline2\"");
    }

    #[test]
    fn csv_escape_empty_string() {
        assert_eq!(csv_escape(""), "");
    }

    // -----------------------------------------------------------------------
    // html_escape
    // -----------------------------------------------------------------------

    #[test]
    fn html_escape_ampersand() {
        assert_eq!(html_escape("a & b"), "a &amp; b");
    }

    #[test]
    fn html_escape_angle_brackets() {
        assert_eq!(html_escape("<div>"), "&lt;div&gt;");
    }

    #[test]
    fn html_escape_quotes() {
        assert_eq!(html_escape("say \"hi\""), "say &quot;hi&quot;");
    }

    #[test]
    fn html_escape_combined() {
        assert_eq!(
            html_escape("<a href=\"url\">A & B</a>"),
            "&lt;a href=&quot;url&quot;&gt;A &amp; B&lt;/a&gt;"
        );
    }

    #[test]
    fn html_escape_no_special_chars() {
        assert_eq!(html_escape("plain text"), "plain text");
    }

    // -----------------------------------------------------------------------
    // delta_class helpers
    // -----------------------------------------------------------------------

    #[test]
    fn delta_class_lower_better_positive_is_pos() {
        assert_eq!(delta_class_lower_better(1.0), "pos");
    }

    #[test]
    fn delta_class_lower_better_negative_is_neg() {
        assert_eq!(delta_class_lower_better(-1.0), "neg");
    }

    #[test]
    fn delta_class_lower_better_zero_is_neutral() {
        assert_eq!(delta_class_lower_better(0.0), "neutral");
    }

    #[test]
    fn delta_class_higher_better_positive_is_neg() {
        // Higher is better, so positive delta = "neg" (green = good)
        assert_eq!(delta_class_higher_better(1.0), "neg");
    }

    #[test]
    fn delta_class_higher_better_negative_is_pos() {
        assert_eq!(delta_class_higher_better(-1.0), "pos");
    }

    #[test]
    fn delta_class_neutral_always_neutral() {
        assert_eq!(delta_class_neutral(100.0), "neutral");
        assert_eq!(delta_class_neutral(-100.0), "neutral");
        assert_eq!(delta_class_neutral(0.0), "neutral");
    }

    // -----------------------------------------------------------------------
    // export_comparison_html
    // -----------------------------------------------------------------------

    #[test]
    fn export_comparison_html_is_valid_html() {
        let cmp = make_comparison();
        let html = export_comparison_html(&cmp);
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("<html"));
        assert!(html.contains("</html>"));
        assert!(html.contains("Comparison Report"));
    }

    #[test]
    fn export_comparison_html_contains_plan_names() {
        let cmp = make_comparison();
        let html = export_comparison_html(&cmp);
        assert!(html.contains("Plan A"));
        assert!(html.contains("Plan B"));
    }

    #[test]
    fn export_comparison_html_contains_metrics() {
        let cmp = make_comparison();
        let html = export_comparison_html(&cmp);
        assert!(html.contains("Total Requests"));
        assert!(html.contains("Mean Response"));
        assert!(html.contains("Throughput"));
        assert!(html.contains("Error Rate"));
    }

    // Helper for comparison tests
    fn make_comparison() -> ComparisonResult {
        use chrono::Utc;
        use uuid::Uuid;
        use crate::results::TestSummary;

        let now = Utc::now();
        let summary_a = TestSummary {
            plan_id: Uuid::new_v4(),
            plan_name: "Plan A".to_string(),
            started_at: now,
            finished_at: now,
            total_requests: 100,
            successful_requests: 95,
            failed_requests: 5,
            min_response_ms: 10,
            max_response_ms: 500,
            mean_response_ms: 100.0,
            p50_response_ms: 80,
            p95_response_ms: 300,
            p99_response_ms: 450,
            requests_per_second: 20.0,
            total_bytes_received: 102400,
        };
        let summary_b = TestSummary {
            plan_id: Uuid::new_v4(),
            plan_name: "Plan B".to_string(),
            started_at: now,
            finished_at: now,
            total_requests: 150,
            successful_requests: 145,
            failed_requests: 5,
            min_response_ms: 8,
            max_response_ms: 400,
            mean_response_ms: 80.0,
            p50_response_ms: 60,
            p95_response_ms: 250,
            p99_response_ms: 380,
            requests_per_second: 30.0,
            total_bytes_received: 153600,
        };

        ComparisonResult {
            run_a: summary_a,
            run_b: summary_b,
            delta_total_requests: 50,
            delta_mean_ms: -20.0,
            delta_p95_ms: -50,
            delta_p99_ms: -70,
            delta_rps: 10.0,
            delta_error_rate: -0.017,
        }
    }
}
