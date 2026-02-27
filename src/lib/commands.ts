import { invoke } from "@tauri-apps/api/core";
import type {
  SendRequestInput,
  SendRequestOutput,
  HistoryEntry,
  WebSocketStep,
  WebSocketResult,
} from "@/types/request";
import type {
  TestPlan,
  PlanSummary,
  ThreadGroup,
  HttpRequest,
  ThreadGroupUpdate,
  HttpRequestUpdate,
  Assertion,
  AssertionRule,
  Variable,
  Extractor,
  ExtractorRule,
  CsvDataSource,
  CsvSharingMode,
} from "@/types/plan";

// ----------------------------------------------------------------
// HTTP Request commands
// ----------------------------------------------------------------

export async function sendRequest(
  input: SendRequestInput
): Promise<SendRequestOutput> {
  return invoke<SendRequestOutput>("send_request", { input });
}

export async function getRequestHistory(): Promise<HistoryEntry[]> {
  return invoke<HistoryEntry[]>("get_request_history");
}

export async function clearRequestHistory(): Promise<void> {
  return invoke<void>("clear_request_history");
}

// ----------------------------------------------------------------
// Test Plan CRUD commands
// ----------------------------------------------------------------

export async function createPlan(name: string): Promise<TestPlan> {
  return invoke<TestPlan>("create_plan", { name });
}

export async function getPlan(id: string): Promise<TestPlan> {
  return invoke<TestPlan>("get_plan", { id });
}

export async function listPlans(): Promise<PlanSummary[]> {
  return invoke<PlanSummary[]>("list_plans");
}

export async function deletePlan(id: string): Promise<void> {
  return invoke<void>("delete_plan", { id });
}

export async function setActivePlan(id: string): Promise<void> {
  return invoke<void>("set_active_plan", { id });
}

export async function getActivePlan(): Promise<TestPlan | null> {
  return invoke<TestPlan | null>("get_active_plan");
}

// ----------------------------------------------------------------
// File I/O commands
// ----------------------------------------------------------------

export async function savePlan(id: string, path: string): Promise<void> {
  return invoke<void>("save_plan", { id, path });
}

export async function loadPlan(path: string): Promise<TestPlan> {
  return invoke<TestPlan>("load_plan", { path });
}

// ----------------------------------------------------------------
// Thread group commands
// ----------------------------------------------------------------

export async function addThreadGroup(
  planId: string,
  name: string
): Promise<ThreadGroup> {
  return invoke<ThreadGroup>("add_thread_group", { planId, name });
}

export async function removeThreadGroup(
  planId: string,
  groupId: string
): Promise<void> {
  return invoke<void>("remove_thread_group", { planId, groupId });
}

export async function updateThreadGroup(
  planId: string,
  groupId: string,
  update: ThreadGroupUpdate
): Promise<ThreadGroup> {
  return invoke<ThreadGroup>("update_thread_group", { planId, groupId, update });
}

// ----------------------------------------------------------------
// HTTP Request commands (within plans)
// ----------------------------------------------------------------

export async function addRequest(
  planId: string,
  groupId: string,
  name: string
): Promise<HttpRequest> {
  return invoke<HttpRequest>("add_request", { planId, groupId, name });
}

export async function removeRequest(
  planId: string,
  groupId: string,
  requestId: string
): Promise<void> {
  return invoke<void>("remove_request", { planId, groupId, requestId });
}

export async function updateRequest(
  planId: string,
  groupId: string,
  requestId: string,
  update: HttpRequestUpdate
): Promise<HttpRequest> {
  return invoke<HttpRequest>("update_request", { planId, groupId, requestId, update });
}

// ----------------------------------------------------------------
// Utility commands
// ----------------------------------------------------------------

export async function duplicateElement(
  planId: string,
  elementId: string
): Promise<TestPlan> {
  return invoke<TestPlan>("duplicate_element", { planId, elementId });
}

export async function reorderThreadGroups(
  planId: string,
  groupIds: string[]
): Promise<void> {
  return invoke<void>("reorder_thread_groups", { planId, groupIds });
}

export async function reorderRequests(
  planId: string,
  groupId: string,
  requestIds: string[]
): Promise<void> {
  return invoke<void>("reorder_requests", { planId, groupId, requestIds });
}

export async function toggleElement(
  planId: string,
  elementId: string
): Promise<boolean> {
  return invoke<boolean>("toggle_element", { planId, elementId });
}

export async function renameElement(
  planId: string,
  elementId: string,
  newName: string
): Promise<void> {
  return invoke<void>("rename_element", { planId, elementId, newName });
}

// ----------------------------------------------------------------
// Assertion commands
// ----------------------------------------------------------------

export async function addAssertion(
  planId: string,
  groupId: string,
  requestId: string,
  name: string,
  rule: AssertionRule
): Promise<Assertion> {
  return invoke<Assertion>("add_assertion", { planId, groupId, requestId, name, rule });
}

export async function removeAssertion(
  planId: string,
  groupId: string,
  requestId: string,
  assertionId: string
): Promise<void> {
  return invoke<void>("remove_assertion", { planId, groupId, requestId, assertionId });
}

export async function updateAssertion(
  planId: string,
  groupId: string,
  requestId: string,
  assertionId: string,
  name?: string,
  rule?: AssertionRule
): Promise<Assertion> {
  return invoke<Assertion>("update_assertion", { planId, groupId, requestId, assertionId, name, rule });
}

// ----------------------------------------------------------------
// Variable commands
// ----------------------------------------------------------------

export async function addVariable(
  planId: string,
  name: string,
  value: string,
  scope: string
): Promise<Variable> {
  return invoke<Variable>("add_variable", { planId, name, value, scope });
}

export async function removeVariable(
  planId: string,
  variableId: string
): Promise<void> {
  return invoke<void>("remove_variable", { planId, variableId });
}

export async function updateVariable(
  planId: string,
  variableId: string,
  name?: string,
  value?: string,
  scope?: string
): Promise<Variable> {
  return invoke<Variable>("update_variable", { planId, variableId, name, value, scope });
}

// ----------------------------------------------------------------
// Extractor commands
// ----------------------------------------------------------------

export async function addExtractor(
  planId: string,
  groupId: string,
  requestId: string,
  name: string,
  variable: string,
  expression: ExtractorRule
): Promise<Extractor> {
  return invoke<Extractor>("add_extractor", { planId, groupId, requestId, name, variable, expression });
}

export async function removeExtractor(
  planId: string,
  groupId: string,
  requestId: string,
  extractorId: string
): Promise<void> {
  return invoke<void>("remove_extractor", { planId, groupId, requestId, extractorId });
}

export async function updateExtractor(
  planId: string,
  groupId: string,
  requestId: string,
  extractorId: string,
  name?: string,
  variable?: string,
  expression?: ExtractorRule
): Promise<Extractor> {
  return invoke<Extractor>("update_extractor", {
    planId,
    groupId,
    requestId,
    extractorId,
    name,
    variable,
    expression,
  });
}

// ----------------------------------------------------------------
// CSV Data Source commands
// ----------------------------------------------------------------

export async function addCsvDataSource(
  planId: string,
  name: string,
  csvContent: string,
  delimiter?: string
): Promise<CsvDataSource> {
  return invoke<CsvDataSource>("add_csv_data_source", { planId, name, csvContent, delimiter });
}

export async function removeCsvDataSource(
  planId: string,
  sourceId: string
): Promise<void> {
  return invoke<void>("remove_csv_data_source", { planId, sourceId });
}

export async function updateCsvDataSource(
  planId: string,
  sourceId: string,
  name?: string,
  sharingMode?: CsvSharingMode,
  recycle?: boolean
): Promise<CsvDataSource> {
  return invoke<CsvDataSource>("update_csv_data_source", { planId, sourceId, name, sharingMode, recycle });
}

// ----------------------------------------------------------------
// Template commands
// ----------------------------------------------------------------

export async function createFromTemplate(template: string): Promise<TestPlan> {
  return invoke<TestPlan>("create_from_template", { template });
}

// ----------------------------------------------------------------
// Engine commands
// ----------------------------------------------------------------

export async function startTest(planId: string): Promise<void> {
  return invoke<void>("start_test", { planId });
}

export async function stopTest(): Promise<void> {
  return invoke<void>("stop_test");
}

export async function forceStopTest(): Promise<void> {
  return invoke<void>("force_stop_test");
}

export async function getEngineStatus(): Promise<import("@/types/engine").EngineStatusKind> {
  return invoke<import("@/types/engine").EngineStatusKind>("get_engine_status");
}

// ----------------------------------------------------------------
// Time series commands
// ----------------------------------------------------------------

export interface TimeBucketEntry {
  second: number;
  requests: number;
  errors: number;
  avg_ms: number;
  min_ms: number;
  max_ms: number;
}

export async function getTimeSeries(): Promise<TimeBucketEntry[]> {
  return invoke<TimeBucketEntry[]>("get_time_series");
}

// ----------------------------------------------------------------
// Export & Reports commands
// ----------------------------------------------------------------

export async function listResults(): Promise<import("@/types/results").ResultSummaryEntry[]> {
  return invoke<import("@/types/results").ResultSummaryEntry[]>("list_results");
}

export async function getResult(runId: string): Promise<import("@/types/results").TestRunResult> {
  return invoke<import("@/types/results").TestRunResult>("get_result", { runId });
}

export async function exportResultsCsv(runId: string): Promise<string> {
  return invoke<string>("export_results_csv", { runId });
}

export async function exportResultsJson(runId: string): Promise<string> {
  return invoke<string>("export_results_json", { runId });
}

export async function exportResultsHtml(runId: string): Promise<string> {
  return invoke<string>("export_results_html", { runId });
}

export async function compareResults(
  runIdA: string,
  runIdB: string
): Promise<import("@/types/results").ComparisonResult> {
  return invoke<import("@/types/results").ComparisonResult>("compare_run_results", { runIdA, runIdB });
}

// ----------------------------------------------------------------
// WebSocket commands
// ----------------------------------------------------------------

export async function testWebSocket(
  url: string,
  headers: Record<string, string>,
  steps: WebSocketStep[]
): Promise<WebSocketResult> {
  return invoke<WebSocketResult>("test_websocket", { url, headers, steps });
}

// ----------------------------------------------------------------
// GraphQL commands
// ----------------------------------------------------------------

export async function sendGraphql(
  url: string,
  query: string,
  variables?: unknown,
  operationName?: string,
  headers?: Record<string, string>
): Promise<SendRequestOutput> {
  return invoke<SendRequestOutput>("send_graphql", {
    url,
    query,
    variables,
    operationName,
    headers: headers ?? {},
  });
}

export async function graphqlIntrospect(
  url: string,
  headers?: Record<string, string>
): Promise<unknown> {
  return invoke<unknown>("graphql_introspect", { url, headers: headers ?? {} });
}
