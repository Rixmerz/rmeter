export type HttpMethod =
  | "GET"
  | "POST"
  | "PUT"
  | "DELETE"
  | "PATCH"
  | "HEAD"
  | "OPTIONS";

// ----------------------------------------------------------------
// WebSocket types
// ----------------------------------------------------------------

export type WebSocketStep =
  | { type: "connect"; url: string; headers: Record<string, string> }
  | { type: "send_text"; message: string }
  | { type: "send_binary"; data: string }
  | { type: "receive"; timeout_ms: number }
  | { type: "delay"; duration_ms: number }
  | { type: "close" };

export interface WebSocketStepResult {
  step_index: number;
  step_type: string;
  elapsed_ms: number;
  success: boolean;
  message: string | null;
  error: string | null;
}

export interface WebSocketResult {
  step_results: WebSocketStepResult[];
  total_elapsed_ms: number;
  connected: boolean;
  error: string | null;
}

export interface Auth {
  type: "bearer" | "basic";
  token?: string;
  username?: string;
  password?: string;
}

export type RequestBody =
  | { type: "json"; json: string }
  | { type: "form_data"; form_data: [string, string][] }
  | { type: "raw"; raw: string }
  | { type: "xml"; xml: string };

export interface SendRequestInput {
  method: string;
  url: string;
  headers: Record<string, string>;
  body: RequestBody | null;
  auth?: Auth | null;
}

export interface SendRequestOutput {
  status: number;
  headers: Record<string, string>;
  body: string;
  elapsed_ms: number;
  size_bytes: number;
}

export interface HistoryEntry {
  id: string;
  input: SendRequestInput;
  output: SendRequestOutput;
  timestamp: string;
}
