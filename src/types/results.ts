export interface AssertionResult {
  assertion_id: string;
  assertion_name: string;
  passed: boolean;
  message: string;
}

export interface ExtractionResult {
  extractor_id: string;
  extractor_name: string;
  variable_name: string;
  success: boolean;
  extracted_value: string | null;
  message: string;
}

export interface RequestResultEvent {
  id: string;
  plan_id: string;
  thread_group_name: string;
  request_name: string;
  timestamp: string;
  status_code: number;
  elapsed_ms: number;
  size_bytes: number;
  assertions_passed: boolean;
  error: string | null;
  assertion_results: AssertionResult[];
  extraction_results: ExtractionResult[];
  /** HTTP method used (e.g. "GET", "POST") */
  method: string;
  /** The resolved URL that was requested */
  url: string;
  /** Response headers (lowercased keys) */
  response_headers?: Record<string, string>;
  /** Response body (truncated to 4 KB) */
  response_body?: string | null;
}

export interface TestSummary {
  plan_id: string;
  plan_name: string;
  started_at: string;
  finished_at: string;
  total_requests: number;
  successful_requests: number;
  failed_requests: number;
  min_response_ms: number;
  max_response_ms: number;
  mean_response_ms: number;
  p50_response_ms: number;
  p95_response_ms: number;
  p99_response_ms: number;
  requests_per_second: number;
  total_bytes_received: number;
}

// ----------------------------------------------------------------
// Export & Reports types
// ----------------------------------------------------------------

export interface ResultSummaryEntry {
  run_id: string;
  plan_name: string;
  started_at: string;
  finished_at: string;
  total_requests: number;
  requests_per_second: number;
  mean_response_ms: number;
  error_rate: number;
}

export interface TestRunResult {
  run_id: string;
  summary: TestSummary;
  time_series: import("@/lib/commands").TimeBucketEntry[];
  request_results: RequestResultEvent[];
}

export interface ComparisonResult {
  run_a: TestSummary;
  run_b: TestSummary;
  delta_total_requests: number;
  delta_mean_ms: number;
  delta_p95_ms: number;
  delta_p99_ms: number;
  delta_rps: number;
  delta_error_rate: number;
}
