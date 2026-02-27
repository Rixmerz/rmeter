// LoopCount variants matching the Rust enum
export type LoopCount =
  | { type: "finite"; count: number }
  | { type: "duration"; seconds: number }
  | { type: "infinite" };

// RequestBody variants matching the Rust enum
export type RequestBody =
  | { type: "json"; json: string }
  | { type: "form_data"; form_data: [string, string][] }
  | { type: "raw"; raw: string }
  | { type: "xml"; xml: string };

// AssertionRule tagged union matching Rust serde enum
export type AssertionRule =
  | { type: "status_code_equals"; expected: number }
  | { type: "status_code_not_equals"; not_expected: number }
  | { type: "status_code_range"; min: number; max: number }
  | { type: "body_contains"; substring: string }
  | { type: "body_not_contains"; substring: string }
  | { type: "json_path"; expression: string; expected: unknown }
  | { type: "response_time_below"; threshold_ms: number }
  | { type: "header_equals"; header: string; expected: string }
  | { type: "header_contains"; header: string; substring: string };

// Assertion matching the Rust struct
export interface Assertion {
  id: string;
  name: string;
  rule: AssertionRule;
}

// ExtractorRule tagged union matching Rust serde enum
export type ExtractorRule =
  | { type: "json_path"; expression: string }
  | { type: "regex"; pattern: string; group: number }
  | { type: "header"; name: string };

// Extractor matching the Rust struct (snake_case serde output)
export interface Extractor {
  id: string;
  name: string;
  variable: string;
  expression: ExtractorRule;
}

// Variable matching the Rust struct
export interface Variable {
  id: string;
  name: string;
  value: string;
  scope: "global" | "plan" | "thread_group";
}

// CSV Data Source matching the Rust struct
export type CsvSharingMode = "all_threads" | "per_thread";

export interface CsvDataSource {
  id: string;
  name: string;
  columns: string[];
  rows: string[][];
  sharing_mode: CsvSharingMode;
  recycle: boolean;
}

// HttpRequest matching the Rust struct (snake_case)
export interface HttpRequest {
  id: string;
  name: string;
  method: string;
  url: string;
  headers: Record<string, string>;
  body: RequestBody | null;
  assertions: Assertion[];
  extractors: Extractor[];
  enabled: boolean;
}

// ThreadGroup matching the Rust struct (snake_case)
export interface ThreadGroup {
  id: string;
  name: string;
  num_threads: number;
  ramp_up_seconds: number;
  loop_count: LoopCount;
  requests: HttpRequest[];
  enabled: boolean;
}

// TestPlan matching the Rust struct (snake_case)
export interface TestPlan {
  id: string;
  name: string;
  description: string;
  thread_groups: ThreadGroup[];
  variables: Variable[];
  csv_data_sources: CsvDataSource[];
  format_version: number;
}

// PlanSummary returned by list_plans
export interface PlanSummary {
  id: string;
  name: string;
  thread_group_count: number;
  request_count: number;
}

// Update types for partial edits
export interface ThreadGroupUpdate {
  name?: string;
  num_threads?: number;
  ramp_up_seconds?: number;
  loop_count?: LoopCount;
  enabled?: boolean;
}

export interface HttpRequestUpdate {
  name?: string;
  method?: string;
  url?: string;
  headers?: Record<string, string>;
  body?: RequestBody | null;
  enabled?: boolean;
}
