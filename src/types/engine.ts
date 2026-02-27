export type EngineStatusKind =
  | "idle"
  | "running"
  | "stopping"
  | "completed"
  | "error";

export interface EngineStatus {
  kind: EngineStatusKind;
  plan_id: string | null;
  started_at: string | null;
  completed_at: string | null;
  error: string | null;
  total_requests: number;
  completed_requests: number;
  failed_requests: number;
}

export interface ProgressEvent {
  completed_requests: number;
  total_errors: number;
  active_threads: number;
  elapsed_ms: number;
  current_rps: number;
  // NEW fields being added by backend agent:
  mean_ms: number;
  p95_ms: number;
  min_ms: number;
  max_ms: number;
}

export interface StatusChangeEvent {
  status: EngineStatusKind;
}
