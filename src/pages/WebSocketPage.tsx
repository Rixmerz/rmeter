import { useState } from "react";
import {
  Plus,
  X,
  Loader2,
  ChevronUp,
  ChevronDown,
  CheckCircle2,
  XCircle,
  Zap,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/cn";
import { testWebSocket } from "@/lib/commands";
import type { WebSocketStep, WebSocketStepResult, WebSocketResult } from "@/types/request";

// ----------------------------------------------------------------
// Helpers / shared styling
// ----------------------------------------------------------------

function inputClass(extra?: string) {
  return cn(
    "h-8 px-3 rounded-md border border-input bg-background text-sm",
    "placeholder:text-muted-foreground",
    "focus:outline-none focus:ring-1 focus:ring-ring",
    extra
  );
}

function numberInputClass(extra?: string) {
  return cn(
    "h-8 w-28 px-3 rounded-md border border-input bg-background text-sm",
    "placeholder:text-muted-foreground",
    "focus:outline-none focus:ring-1 focus:ring-ring",
    extra
  );
}

type StepType = WebSocketStep["type"];

const STEP_LABELS: Record<StepType, string> = {
  connect: "Connect",
  send_text: "Send Text",
  send_binary: "Send Binary",
  receive: "Receive",
  delay: "Delay",
  close: "Close",
};

const STEP_COLORS: Record<StepType, string> = {
  connect: "text-green-600 dark:text-green-400",
  send_text: "text-blue-600 dark:text-blue-400",
  send_binary: "text-purple-600 dark:text-purple-400",
  receive: "text-yellow-600 dark:text-yellow-400",
  delay: "text-orange-600 dark:text-orange-400",
  close: "text-red-600 dark:text-red-400",
};

// ----------------------------------------------------------------
// Editable step state (mirrors WebSocketStep but all fields optional
// to accommodate partial UI state before finalising)
// ----------------------------------------------------------------

interface StepEntry {
  id: string;
  type: StepType;
  // connect
  connectUrl: string;
  connectHeaders: { id: string; key: string; value: string }[];
  // send_text
  message: string;
  // send_binary
  binaryData: string;
  // receive
  timeoutMs: number;
  // delay
  durationMs: number;
}

function makeDefaultStep(type: StepType): StepEntry {
  return {
    id: crypto.randomUUID(),
    type,
    connectUrl: "",
    connectHeaders: [],
    message: "",
    binaryData: "",
    timeoutMs: 5000,
    durationMs: 1000,
  };
}

function stepEntryToWsStep(entry: StepEntry): WebSocketStep {
  switch (entry.type) {
    case "connect":
      return {
        type: "connect",
        url: entry.connectUrl,
        headers: Object.fromEntries(
          entry.connectHeaders
            .filter((h) => h.key.trim() !== "")
            .map((h) => [h.key.trim(), h.value.trim()])
        ),
      };
    case "send_text":
      return { type: "send_text", message: entry.message };
    case "send_binary":
      return { type: "send_binary", data: entry.binaryData };
    case "receive":
      return { type: "receive", timeout_ms: entry.timeoutMs };
    case "delay":
      return { type: "delay", duration_ms: entry.durationMs };
    case "close":
      return { type: "close" };
  }
}

// ----------------------------------------------------------------
// Header key-value editor (reusable within steps)
// ----------------------------------------------------------------

interface HeaderEditorProps {
  headers: { id: string; key: string; value: string }[];
  onChange: (headers: { id: string; key: string; value: string }[]) => void;
  label?: string;
}

function HeaderEditor({ headers, onChange, label = "Headers" }: HeaderEditorProps) {
  const addRow = () =>
    onChange([...headers, { id: crypto.randomUUID(), key: "", value: "" }]);

  const updateRow = (id: string, key: string, value: string) =>
    onChange(headers.map((h) => (h.id === id ? { ...h, key, value } : h)));

  const removeRow = (id: string) => onChange(headers.filter((h) => h.id !== id));

  return (
    <div className="space-y-2">
      <div className="flex items-center justify-between">
        <span className="text-xs text-muted-foreground font-medium">{label}</span>
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={addRow}
          className="h-6 gap-1 text-xs"
        >
          <Plus className="h-3 w-3" />
          Add
        </Button>
      </div>
      {headers.length === 0 ? (
        <p className="text-xs text-muted-foreground italic">No headers.</p>
      ) : (
        headers.map((h) => (
          <div key={h.id} className="flex gap-2 items-center">
            <input
              type="text"
              value={h.key}
              onChange={(e) => updateRow(h.id, e.target.value, h.value)}
              placeholder="Header name"
              aria-label="Header name"
              className={inputClass("flex-1")}
            />
            <input
              type="text"
              value={h.value}
              onChange={(e) => updateRow(h.id, h.key, e.target.value)}
              placeholder="Value"
              aria-label="Header value"
              className={inputClass("flex-1")}
            />
            <Button
              type="button"
              variant="ghost"
              size="icon"
              onClick={() => removeRow(h.id)}
              aria-label="Remove header"
              className="h-8 w-8 shrink-0 text-muted-foreground hover:text-destructive"
            >
              <X className="h-3.5 w-3.5" />
            </Button>
          </div>
        ))
      )}
    </div>
  );
}

// ----------------------------------------------------------------
// Individual step editor card
// ----------------------------------------------------------------

interface StepCardProps {
  step: StepEntry;
  index: number;
  total: number;
  onChange: (updated: StepEntry) => void;
  onRemove: () => void;
  onMoveUp: () => void;
  onMoveDown: () => void;
}

function StepCard({
  step,
  index,
  total,
  onChange,
  onRemove,
  onMoveUp,
  onMoveDown,
}: StepCardProps) {
  const update = (patch: Partial<StepEntry>) => onChange({ ...step, ...patch });

  return (
    <div className="rounded-lg border border-border bg-card">
      {/* Card header */}
      <div className="flex items-center gap-2 px-3 py-2 border-b border-border">
        <span className="text-xs font-mono text-muted-foreground w-5 shrink-0 text-right">
          {index + 1}
        </span>

        <select
          value={step.type}
          onChange={(e) =>
            onChange({
              ...makeDefaultStep(e.target.value as StepType),
              id: step.id,
            })
          }
          aria-label={`Step ${index + 1} type`}
          className={cn(
            "h-7 pl-2 pr-6 rounded border border-input bg-background text-xs font-semibold",
            "focus:outline-none focus:ring-1 focus:ring-ring appearance-none cursor-pointer",
            STEP_COLORS[step.type]
          )}
        >
          {(Object.keys(STEP_LABELS) as StepType[]).map((t) => (
            <option key={t} value={t} className="text-foreground font-normal">
              {STEP_LABELS[t]}
            </option>
          ))}
        </select>

        <div className="ml-auto flex items-center gap-1">
          <Button
            type="button"
            variant="ghost"
            size="icon"
            onClick={onMoveUp}
            disabled={index === 0}
            aria-label="Move step up"
            className="h-7 w-7 text-muted-foreground"
          >
            <ChevronUp className="h-3.5 w-3.5" />
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            onClick={onMoveDown}
            disabled={index === total - 1}
            aria-label="Move step down"
            className="h-7 w-7 text-muted-foreground"
          >
            <ChevronDown className="h-3.5 w-3.5" />
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            onClick={onRemove}
            aria-label="Remove step"
            className="h-7 w-7 text-muted-foreground hover:text-destructive"
          >
            <X className="h-3.5 w-3.5" />
          </Button>
        </div>
      </div>

      {/* Card body — type-specific fields */}
      <div className="p-3 space-y-3">
        {step.type === "connect" && (
          <>
            <div className="flex items-center gap-2">
              <label className="text-xs text-muted-foreground w-16 shrink-0">URL</label>
              <input
                type="text"
                value={step.connectUrl}
                onChange={(e) => update({ connectUrl: e.target.value })}
                placeholder="wss://example.com/socket"
                aria-label="WebSocket connect URL"
                className={inputClass("flex-1")}
              />
            </div>
            <HeaderEditor
              headers={step.connectHeaders}
              onChange={(connectHeaders) => update({ connectHeaders })}
              label="Connect Headers"
            />
          </>
        )}

        {step.type === "send_text" && (
          <div className="space-y-1">
            <label className="text-xs text-muted-foreground">Message</label>
            <textarea
              value={step.message}
              onChange={(e) => update({ message: e.target.value })}
              placeholder='{"event": "subscribe", "channel": "ticker"}'
              aria-label="Text message to send"
              rows={3}
              className={cn(
                "w-full px-3 py-2 rounded-md border border-input bg-background text-sm font-mono resize-y",
                "placeholder:text-muted-foreground",
                "focus:outline-none focus:ring-1 focus:ring-ring"
              )}
            />
          </div>
        )}

        {step.type === "send_binary" && (
          <div className="space-y-1">
            <label className="text-xs text-muted-foreground">Data (base64)</label>
            <textarea
              value={step.binaryData}
              onChange={(e) => update({ binaryData: e.target.value })}
              placeholder="SGVsbG8gV29ybGQ="
              aria-label="Binary data to send (base64 encoded)"
              rows={3}
              className={cn(
                "w-full px-3 py-2 rounded-md border border-input bg-background text-sm font-mono resize-y",
                "placeholder:text-muted-foreground",
                "focus:outline-none focus:ring-1 focus:ring-ring"
              )}
            />
          </div>
        )}

        {step.type === "receive" && (
          <div className="flex items-center gap-2">
            <label className="text-xs text-muted-foreground w-24 shrink-0">Timeout (ms)</label>
            <input
              type="number"
              value={step.timeoutMs}
              onChange={(e) => update({ timeoutMs: Math.max(0, Number(e.target.value)) })}
              min={0}
              aria-label="Receive timeout in milliseconds"
              className={numberInputClass()}
            />
          </div>
        )}

        {step.type === "delay" && (
          <div className="flex items-center gap-2">
            <label className="text-xs text-muted-foreground w-24 shrink-0">Duration (ms)</label>
            <input
              type="number"
              value={step.durationMs}
              onChange={(e) => update({ durationMs: Math.max(0, Number(e.target.value)) })}
              min={0}
              aria-label="Delay duration in milliseconds"
              className={numberInputClass()}
            />
          </div>
        )}

        {step.type === "close" && (
          <p className="text-xs text-muted-foreground italic">
            Closes the WebSocket connection.
          </p>
        )}
      </div>
    </div>
  );
}

// ----------------------------------------------------------------
// Results panel
// ----------------------------------------------------------------

interface ResultsPanelProps {
  result: WebSocketResult;
}

function StepResultRow({ r }: { r: WebSocketStepResult }) {
  const [expanded, setExpanded] = useState(false);
  const hasDetail = r.message !== null || r.error !== null;

  return (
    <div
      className={cn(
        "rounded-md border px-3 py-2 space-y-1",
        r.success
          ? "border-green-500/30 bg-green-500/5"
          : "border-red-500/30 bg-red-500/5"
      )}
    >
      <div className="flex items-center gap-2">
        {r.success ? (
          <CheckCircle2 className="h-4 w-4 text-green-500 shrink-0" aria-hidden />
        ) : (
          <XCircle className="h-4 w-4 text-red-500 shrink-0" aria-hidden />
        )}

        <span className="text-xs font-mono text-muted-foreground w-5 shrink-0">
          {r.step_index + 1}
        </span>

        <span className="text-sm font-medium flex-1">
          {STEP_LABELS[r.step_type as StepType] ?? r.step_type}
        </span>

        <span className="text-xs text-muted-foreground shrink-0">{r.elapsed_ms}ms</span>

        {hasDetail && (
          <button
            type="button"
            onClick={() => setExpanded((v) => !v)}
            className="text-xs text-muted-foreground hover:text-foreground transition-colors shrink-0"
            aria-expanded={expanded}
            aria-label="Toggle step detail"
          >
            {expanded ? "hide" : "show"}
          </button>
        )}
      </div>

      {expanded && hasDetail && (
        <pre className="text-xs font-mono bg-muted/50 rounded p-2 overflow-auto max-h-40 whitespace-pre-wrap break-all mt-1">
          {r.error ?? r.message}
        </pre>
      )}
    </div>
  );
}

function ResultsPanel({ result }: ResultsPanelProps) {
  return (
    <div className="rounded-lg border border-border bg-card flex flex-col h-full">
      {/* Panel header */}
      <div className="flex items-center gap-3 px-4 py-3 border-b border-border shrink-0">
        <h2 className="text-sm font-semibold">Results</h2>

        <div className="ml-auto flex items-center gap-3 text-xs text-muted-foreground">
          {result.connected ? (
            <span className="text-green-600 dark:text-green-400 font-medium">Connected</span>
          ) : (
            <span className="text-red-600 dark:text-red-400 font-medium">Failed</span>
          )}
          <span>{result.total_elapsed_ms}ms total</span>
        </div>
      </div>

      {/* Top-level error */}
      {result.error && (
        <div className="mx-4 mt-3 rounded-md border border-destructive/50 bg-destructive/10 px-3 py-2">
          <p className="text-sm text-destructive font-medium">Connection error</p>
          <p className="text-xs text-destructive/80 mt-0.5 font-mono">{result.error}</p>
        </div>
      )}

      {/* Step results list */}
      <div className="flex-1 overflow-y-auto p-4 space-y-2">
        {result.step_results.length === 0 ? (
          <p className="text-sm text-muted-foreground text-center py-8">
            No step results to display.
          </p>
        ) : (
          result.step_results.map((r) => (
            <StepResultRow key={r.step_index} r={r} />
          ))
        )}
      </div>

      {/* Footer summary */}
      <div className="px-4 py-3 border-t border-border shrink-0 flex items-center gap-4 text-xs text-muted-foreground">
        <span>
          {result.step_results.filter((r) => r.success).length} /{" "}
          {result.step_results.length} steps passed
        </span>
        <span className="ml-auto">Total: {result.total_elapsed_ms}ms</span>
      </div>
    </div>
  );
}

// ----------------------------------------------------------------
// Global headers editor for the top-level connection (page level)
// ----------------------------------------------------------------

interface GlobalHeaderEditorProps {
  headers: { id: string; key: string; value: string }[];
  onChange: (h: { id: string; key: string; value: string }[]) => void;
}

function GlobalHeaderEditor({ headers, onChange }: GlobalHeaderEditorProps) {
  const addRow = () =>
    onChange([...headers, { id: crypto.randomUUID(), key: "", value: "" }]);
  const updateRow = (id: string, key: string, value: string) =>
    onChange(headers.map((h) => (h.id === id ? { ...h, key, value } : h)));
  const removeRow = (id: string) => onChange(headers.filter((h) => h.id !== id));

  return (
    <div className="rounded-lg border border-border bg-card">
      <div className="flex items-center justify-between px-4 py-3 border-b border-border">
        <h2 className="text-sm font-semibold">
          Global Headers{" "}
          {headers.length > 0 && (
            <span className="text-xs text-muted-foreground font-normal">
              ({headers.length})
            </span>
          )}
        </h2>
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={addRow}
          className="h-7 gap-1 text-xs"
        >
          <Plus className="h-3 w-3" />
          Add Header
        </Button>
      </div>
      <div className="p-3 space-y-2">
        {headers.length === 0 ? (
          <p className="text-sm text-muted-foreground text-center py-3">
            No global headers. These are merged into every Connect step.
          </p>
        ) : (
          headers.map((h) => (
            <div key={h.id} className="flex gap-2 items-center">
              <input
                type="text"
                value={h.key}
                onChange={(e) => updateRow(h.id, e.target.value, h.value)}
                placeholder="Header name"
                aria-label="Header name"
                className={inputClass("flex-1")}
              />
              <input
                type="text"
                value={h.value}
                onChange={(e) => updateRow(h.id, h.key, e.target.value)}
                placeholder="Header value"
                aria-label="Header value"
                className={inputClass("flex-1")}
              />
              <Button
                type="button"
                variant="ghost"
                size="icon"
                onClick={() => removeRow(h.id)}
                aria-label="Remove header"
                className="h-8 w-8 text-muted-foreground hover:text-destructive"
              >
                <X className="h-3.5 w-3.5" />
              </Button>
            </div>
          ))
        )}
      </div>
    </div>
  );
}

// ----------------------------------------------------------------
// Main page
// ----------------------------------------------------------------

export function WebSocketPage() {
  const [url, setUrl] = useState("wss://");
  const [globalHeaders, setGlobalHeaders] = useState<
    { id: string; key: string; value: string }[]
  >([]);
  const [steps, setSteps] = useState<StepEntry[]>([
    makeDefaultStep("connect"),
    makeDefaultStep("send_text"),
    makeDefaultStep("receive"),
    makeDefaultStep("close"),
  ]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<WebSocketResult | null>(null);
  const [newStepType, setNewStepType] = useState<StepType>("send_text");

  // ---- step mutations ----
  const addStep = (type: StepType) =>
    setSteps((prev) => [...prev, makeDefaultStep(type)]);

  const updateStep = (index: number, updated: StepEntry) =>
    setSteps((prev) => prev.map((s, i) => (i === index ? updated : s)));

  const removeStep = (index: number) =>
    setSteps((prev) => prev.filter((_, i) => i !== index));

  const moveStep = (from: number, to: number) => {
    if (to < 0 || to >= steps.length) return;
    setSteps((prev) => {
      const next = [...prev];
      const [item] = next.splice(from, 1);
      next.splice(to, 0, item);
      return next;
    });
  };

  // ---- execution ----
  const handleRun = async () => {
    if (!url.trim()) {
      setError("WebSocket URL is required.");
      return;
    }

    const headers = Object.fromEntries(
      globalHeaders
        .filter((h) => h.key.trim() !== "")
        .map((h) => [h.key.trim(), h.value.trim()])
    );

    const wsSteps: WebSocketStep[] = steps.map(stepEntryToWsStep);

    setLoading(true);
    setError(null);
    setResult(null);

    try {
      const res = await testWebSocket(url.trim(), headers, wsSteps);
      setResult(res);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="h-full flex flex-col gap-0 min-h-0 p-6">
      {/* Page header */}
      <div className="flex items-start justify-between shrink-0 mb-5">
        <div>
          <h1 className="text-2xl font-bold tracking-tight flex items-center gap-2">
            <Zap className="h-6 w-6 text-yellow-500" aria-hidden />
            WebSocket Tester
          </h1>
          <p className="text-sm text-muted-foreground mt-1">
            Build and execute WebSocket step sequences
          </p>
        </div>
      </div>

      {/* Two-panel layout */}
      <div className="flex gap-4 flex-1 min-h-0">
        {/* Left panel — step builder */}
        <div className="flex flex-col gap-4 w-[55%] min-h-0 overflow-y-auto pr-1">
          {/* URL + run */}
          <div className="rounded-lg border border-border bg-card p-3 space-y-3 shrink-0">
            <div className="flex gap-2">
              <input
                type="text"
                value={url}
                onChange={(e) => setUrl(e.target.value)}
                placeholder="wss://example.com/socket"
                aria-label="WebSocket URL"
                className={cn(
                  "flex-1 h-9 px-3 rounded-md border border-input bg-background text-sm",
                  "placeholder:text-muted-foreground",
                  "focus:outline-none focus:ring-1 focus:ring-ring"
                )}
              />
              <Button
                type="button"
                onClick={() => void handleRun()}
                disabled={loading || !url.trim()}
                className="gap-2 shrink-0"
              >
                {loading ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  <Zap className="h-4 w-4" />
                )}
                {loading ? "Running..." : "Run"}
              </Button>
            </div>
          </div>

          {/* Global headers */}
          <GlobalHeaderEditor headers={globalHeaders} onChange={setGlobalHeaders} />

          {/* Step list */}
          <div className="rounded-lg border border-border bg-card shrink-0">
            <div className="flex items-center justify-between px-4 py-3 border-b border-border">
              <h2 className="text-sm font-semibold">
                Steps{" "}
                {steps.length > 0 && (
                  <span className="text-xs text-muted-foreground font-normal">
                    ({steps.length})
                  </span>
                )}
              </h2>

              <div className="flex items-center gap-1">
                <select
                  value={newStepType}
                  onChange={(e) => setNewStepType(e.target.value as StepType)}
                  aria-label="Step type to add"
                  className={cn(
                    "h-7 pl-2 pr-6 rounded border border-input bg-background text-xs",
                    "focus:outline-none focus:ring-1 focus:ring-ring appearance-none cursor-pointer"
                  )}
                >
                  {(Object.keys(STEP_LABELS) as StepType[]).map((t) => (
                    <option key={t} value={t}>
                      {STEP_LABELS[t]}
                    </option>
                  ))}
                </select>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  className="h-7 gap-1 text-xs"
                  onClick={() => addStep(newStepType)}
                >
                  <Plus className="h-3 w-3" />
                  Add Step
                </Button>
              </div>
            </div>

            <div className="p-3 space-y-2">
              {steps.length === 0 ? (
                <p className="text-sm text-muted-foreground text-center py-6">
                  No steps yet. Use "Add Step" to build your sequence.
                </p>
              ) : (
                steps.map((step, index) => (
                  <StepCard
                    key={step.id}
                    step={step}
                    index={index}
                    total={steps.length}
                    onChange={(updated) => updateStep(index, updated)}
                    onRemove={() => removeStep(index)}
                    onMoveUp={() => moveStep(index, index - 1)}
                    onMoveDown={() => moveStep(index, index + 1)}
                  />
                ))
              )}
            </div>
          </div>
        </div>

        {/* Right panel — results */}
        <div className="flex flex-col flex-1 min-h-0">
          {error && (
            <div
              role="alert"
              className="rounded-lg border border-destructive/50 bg-destructive/10 px-4 py-3 mb-4 shrink-0"
            >
              <p className="text-sm text-destructive font-medium">Execution failed</p>
              <p className="text-sm text-destructive/80 mt-1">{error}</p>
            </div>
          )}

          {result ? (
            <ResultsPanel result={result} />
          ) : (
            <div className="flex-1 rounded-lg border border-border bg-card flex items-center justify-center">
              <div className="text-center space-y-2 p-8">
                <Zap className="h-10 w-10 text-muted-foreground/30 mx-auto" aria-hidden />
                <p className="text-sm text-muted-foreground">
                  Build your step sequence and click Run to see results here.
                </p>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
