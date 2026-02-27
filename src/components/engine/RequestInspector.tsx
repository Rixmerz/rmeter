import { useState, useMemo } from "react";
import {
  ChevronDown,
  ChevronRight,
  CheckCircle,
  XCircle,
  Search,
} from "lucide-react";
import { cn } from "@/lib/cn";
import { useEngineStore } from "@/stores/useEngineStore";
import type { RequestResultEvent } from "@/types/results";

// ----------------------------------------------------------------
// Helpers
// ----------------------------------------------------------------

const METHOD_COLORS: Record<string, string> = {
  GET: "text-green-600 dark:text-green-400",
  POST: "text-blue-600 dark:text-blue-400",
  PUT: "text-yellow-600 dark:text-yellow-400",
  DELETE: "text-red-600 dark:text-red-400",
  PATCH: "text-orange-600 dark:text-orange-400",
  HEAD: "text-purple-600 dark:text-purple-400",
  OPTIONS: "text-cyan-600 dark:text-cyan-400",
};

function getStatusColor(status: number): string {
  if (status >= 500) return "text-red-600 dark:text-red-400";
  if (status >= 400) return "text-yellow-600 dark:text-yellow-400";
  if (status >= 300) return "text-blue-500";
  if (status >= 200) return "text-green-600 dark:text-green-400";
  return "text-muted-foreground";
}

function formatTime(iso: string): string {
  try {
    return new Date(iso).toLocaleTimeString("en-US", {
      hour12: false,
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  } catch {
    return iso;
  }
}

function formatBody(body: string | null | undefined): string {
  if (!body) return "";
  try {
    return JSON.stringify(JSON.parse(body), null, 2);
  } catch {
    return body;
  }
}

// ----------------------------------------------------------------
// Detail panel for a single request
// ----------------------------------------------------------------

interface DetailPanelProps {
  result: RequestResultEvent;
}

function DetailPanel({ result }: DetailPanelProps) {
  const [activeTab, setActiveTab] = useState<"response" | "headers">("response");
  const headers = result.response_headers ?? {};
  const headerCount = Object.keys(headers).length;
  const body = formatBody(result.response_body);

  return (
    <div className="border-t border-border bg-muted/10">
      {/* Sub-tabs */}
      <div className="flex items-center gap-1 px-4 pt-2 pb-1">
        <button
          onClick={() => setActiveTab("response")}
          className={cn(
            "px-2 py-1 text-xs font-medium rounded transition-colors",
            activeTab === "response"
              ? "bg-primary text-primary-foreground"
              : "text-muted-foreground hover:text-foreground hover:bg-muted"
          )}
        >
          Response Body
        </button>
        <button
          onClick={() => setActiveTab("headers")}
          className={cn(
            "px-2 py-1 text-xs font-medium rounded transition-colors",
            activeTab === "headers"
              ? "bg-primary text-primary-foreground"
              : "text-muted-foreground hover:text-foreground hover:bg-muted"
          )}
        >
          Headers {headerCount > 0 && `(${headerCount})`}
        </button>
      </div>

      {/* Content */}
      <div className="px-4 pb-3 pt-1">
        {activeTab === "response" && (
          <div>
            {result.error ? (
              <div className="text-xs text-destructive font-mono bg-destructive/10 rounded p-2">
                {result.error}
              </div>
            ) : body ? (
              <pre className="text-xs font-mono bg-background rounded border border-border p-2 overflow-auto max-h-64 whitespace-pre-wrap break-all">
                {body}
              </pre>
            ) : (
              <p className="text-xs text-muted-foreground italic">No response body</p>
            )}
          </div>
        )}

        {activeTab === "headers" && (
          <div>
            {headerCount === 0 ? (
              <p className="text-xs text-muted-foreground italic">No response headers</p>
            ) : (
              <div className="space-y-0.5">
                {Object.entries(headers).map(([key, value]) => (
                  <div key={key} className="flex gap-2 text-xs font-mono">
                    <span className="text-muted-foreground shrink-0 min-w-[140px]">{key}</span>
                    <span className="text-foreground break-all">{value}</span>
                  </div>
                ))}
              </div>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

// ----------------------------------------------------------------
// Single result row
// ----------------------------------------------------------------

interface InspectorRowProps {
  result: RequestResultEvent;
  index: number;
  isExpanded: boolean;
  onToggle: () => void;
}

function InspectorRow({ result, index, isExpanded, onToggle }: InspectorRowProps) {
  const isSuccess = !result.error;
  const method = result.method || "???";

  return (
    <div className={cn("border-b border-border/30", isExpanded && "bg-muted/20")}>
      <button
        className={cn(
          "w-full flex items-center gap-2 px-3 py-2 text-xs text-left transition-colors",
          "hover:bg-muted/40",
          !isExpanded && index % 2 === 0 && "bg-background",
          !isExpanded && index % 2 !== 0 && "bg-muted/10"
        )}
        onClick={onToggle}
        aria-expanded={isExpanded}
      >
        {/* Expand chevron */}
        <span className="shrink-0 text-muted-foreground">
          {isExpanded ? (
            <ChevronDown className="h-3 w-3" />
          ) : (
            <ChevronRight className="h-3 w-3" />
          )}
        </span>

        {/* Success icon */}
        <span className="shrink-0">
          {isSuccess ? (
            <CheckCircle className="h-3 w-3 text-green-500" />
          ) : (
            <XCircle className="h-3 w-3 text-destructive" />
          )}
        </span>

        {/* Method */}
        <span
          className={cn(
            "shrink-0 w-14 font-mono font-bold",
            METHOD_COLORS[method] ?? "text-foreground"
          )}
        >
          {method}
        </span>

        {/* Status */}
        <span
          className={cn(
            "shrink-0 w-10 text-right tabular-nums font-medium",
            getStatusColor(result.status_code)
          )}
        >
          {result.status_code > 0 ? result.status_code : "—"}
        </span>

        {/* Elapsed */}
        <span className="shrink-0 w-16 text-right tabular-nums text-muted-foreground">
          {result.elapsed_ms}ms
        </span>

        {/* Size */}
        <span className="shrink-0 w-14 text-right tabular-nums text-muted-foreground">
          {result.size_bytes > 0 ? `${(result.size_bytes / 1024).toFixed(1)}K` : "—"}
        </span>

        {/* Time */}
        <span className="shrink-0 w-16 text-right text-muted-foreground">
          {formatTime(result.timestamp)}
        </span>

        {/* URL */}
        <span className="flex-1 min-w-0 truncate font-mono text-foreground/80 ml-2" title={result.url}>
          {result.url || result.request_name}
        </span>
      </button>

      {/* Expanded detail */}
      {isExpanded && <DetailPanel result={result} />}
    </div>
  );
}

// ----------------------------------------------------------------
// RequestInspector
// ----------------------------------------------------------------

export function RequestInspector() {
  const recentResults = useEngineStore((s) => s.recentResults);
  const status = useEngineStore((s) => s.status);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [filter, setFilter] = useState("");

  const filtered = useMemo(() => {
    if (!filter.trim()) return recentResults;
    const q = filter.toLowerCase();
    return recentResults.filter(
      (r) =>
        r.url?.toLowerCase().includes(q) ||
        r.request_name.toLowerCase().includes(q) ||
        r.method?.toLowerCase().includes(q) ||
        String(r.status_code).includes(q)
    );
  }, [recentResults, filter]);

  function toggleRow(id: string) {
    setExpandedId((prev) => (prev === id ? null : id));
  }

  return (
    <div className="flex flex-col flex-1 min-h-0 bg-card">
      {/* Header */}
      <div className="flex items-center gap-2 px-3 py-1.5 border-b border-border shrink-0">
        <span className="text-xs font-medium text-muted-foreground">
          Inspector
          <span className="ml-1.5 px-1.5 py-0.5 rounded bg-muted text-muted-foreground">
            {filtered.length}
          </span>
        </span>

        {/* Filter */}
        <div className="flex-1 max-w-xs ml-auto relative">
          <Search className="absolute left-2 top-1/2 -translate-y-1/2 h-3 w-3 text-muted-foreground pointer-events-none" />
          <input
            type="text"
            value={filter}
            onChange={(e) => setFilter(e.target.value)}
            placeholder="Filter by URL, method, status..."
            aria-label="Filter requests"
            className={cn(
              "w-full text-xs pl-6 pr-2 py-1 rounded border border-input bg-background",
              "focus:outline-none focus:ring-1 focus:ring-ring",
              "placeholder:text-muted-foreground/50"
            )}
          />
        </div>
      </div>

      {/* Column headers */}
      <div
        className="flex items-center gap-2 px-3 py-1 text-[10px] text-muted-foreground uppercase tracking-wider bg-card border-b border-border shrink-0"
        role="row"
      >
        <span className="shrink-0 w-3" />
        <span className="shrink-0 w-3" />
        <span className="shrink-0 w-14">Method</span>
        <span className="shrink-0 w-10 text-right">Status</span>
        <span className="shrink-0 w-16 text-right">Time</span>
        <span className="shrink-0 w-14 text-right">Size</span>
        <span className="shrink-0 w-16 text-right">Clock</span>
        <span className="flex-1 ml-2">URL</span>
      </div>

      {/* List */}
      <div className="overflow-y-auto flex-1 min-h-0">
        {status === "idle" && recentResults.length === 0 ? (
          <div className="flex items-center justify-center py-12 text-sm text-muted-foreground">
            Run a test to inspect request/response details here.
          </div>
        ) : filtered.length === 0 ? (
          <div className="flex items-center justify-center py-12 text-sm text-muted-foreground">
            {filter ? "No requests match the filter." : "Waiting for results..."}
          </div>
        ) : (
          filtered.map((result, i) => (
            <InspectorRow
              key={`${result.id}-${i}`}
              result={result}
              index={i}
              isExpanded={expandedId === result.id}
              onToggle={() => toggleRow(result.id)}
            />
          ))
        )}
      </div>
    </div>
  );
}
