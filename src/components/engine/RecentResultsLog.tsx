import { useEffect, useRef } from "react";
import { CheckCircle, XCircle } from "lucide-react";
import { cn } from "@/lib/cn";
import { useEngineStore } from "@/stores/useEngineStore";
import type { RequestResultEvent } from "@/types/results";

// ----------------------------------------------------------------
// HTTP status code color helper
// ----------------------------------------------------------------

function getStatusColor(status: number, isSuccess: boolean): string {
  if (!isSuccess) return "text-destructive font-medium";
  if (status >= 500) return "text-destructive font-medium";
  if (status >= 400) return "text-yellow-600 dark:text-yellow-400 font-medium";
  if (status >= 300) return "text-blue-500 font-medium";
  if (status >= 200) return "text-green-600 dark:text-green-400 font-medium";
  return "text-muted-foreground";
}

// ----------------------------------------------------------------
// Result row
// ----------------------------------------------------------------

interface ResultRowProps {
  result: RequestResultEvent;
  index: number;
}

function formatTimestamp(iso: string): string {
  try {
    const d = new Date(iso);
    return d.toLocaleTimeString("en-US", {
      hour12: false,
      hour: "2-digit",
      minute: "2-digit",
      second: "2-digit",
    });
  } catch {
    return iso;
  }
}

function ResultRow({ result, index }: ResultRowProps) {
  const isSuccess = !result.error;
  return (
    <div
      className={cn(
        "flex items-center gap-2 px-3 py-1 text-xs font-mono border-b border-border/30",
        "hover:bg-muted/40 transition-colors",
        index % 2 === 0 ? "bg-background" : "bg-muted/10"
      )}
      role="row"
    >
      {/* Success/fail icon */}
      <span className="shrink-0" aria-hidden="true">
        {isSuccess ? (
          <CheckCircle className="h-3 w-3 text-green-500" />
        ) : (
          <XCircle className="h-3 w-3 text-destructive" />
        )}
      </span>

      {/* Timestamp */}
      <span className="text-muted-foreground shrink-0 w-20">
        {formatTimestamp(result.timestamp)}
      </span>

      {/* Request name */}
      <span
        className="truncate flex-1 min-w-0 text-foreground"
        title={result.request_name}
      >
        {result.request_name}
      </span>

      {/* Status code */}
      <span
        className={cn("shrink-0 w-12 text-right tabular-nums", getStatusColor(result.status_code, isSuccess))}
        aria-label={`HTTP status ${result.status_code}`}
      >
        {result.status_code > 0 ? result.status_code : "—"}
      </span>

      {/* Elapsed */}
      <span
        className="shrink-0 w-16 text-right tabular-nums text-muted-foreground"
        aria-label={`${result.elapsed_ms} milliseconds`}
      >
        {result.elapsed_ms} ms
      </span>

      {/* Assertion badge */}
      {(result.assertion_results ?? []).length > 0 && (() => {
        const results = result.assertion_results ?? [];
        const passed = results.filter((r) => r.passed).length;
        const total = results.length;
        const allPassed = passed === total;
        return (
          <span
            className={cn(
              "shrink-0 px-1 py-0.5 rounded text-[10px] font-medium tabular-nums",
              allPassed
                ? "bg-green-500/15 text-green-600 dark:text-green-400"
                : "bg-destructive/15 text-destructive"
            )}
            title={`${passed}/${total} assertions passed`}
            aria-label={`${passed} of ${total} assertions passed`}
          >
            {passed}/{total} assert
          </span>
        );
      })()}

      {/* Extraction badge */}
      {(result.extraction_results ?? []).length > 0 && (() => {
        const results = result.extraction_results ?? [];
        const succeeded = results.filter((r) => r.success).length;
        const total = results.length;
        const allSucceeded = succeeded === total;
        return (
          <span
            className={cn(
              "shrink-0 px-1 py-0.5 rounded text-[10px] font-medium tabular-nums",
              allSucceeded
                ? "bg-blue-500/15 text-blue-600 dark:text-blue-400"
                : "bg-destructive/15 text-destructive"
            )}
            title={`${succeeded}/${total} extractions succeeded`}
            aria-label={`${succeeded} of ${total} extractions succeeded`}
          >
            {succeeded}/{total} extract
          </span>
        );
      })()}

      {/* Error message (if any) */}
      {result.error && (
        <span
          className="shrink-0 text-destructive truncate max-w-32"
          title={result.error}
        >
          {result.error}
        </span>
      )}
    </div>
  );
}

// ----------------------------------------------------------------
// RecentResultsLog
// ----------------------------------------------------------------

export function RecentResultsLog() {
  const recentResults = useEngineStore((s) => s.recentResults);
  const status = useEngineStore((s) => s.status);
  const bottomRef = useRef<HTMLDivElement>(null);

  // Auto-scroll to bottom when new results arrive
  useEffect(() => {
    if (bottomRef.current) {
      bottomRef.current.scrollIntoView({ behavior: "smooth" });
    }
  }, [recentResults.length]);

  return (
    <div
      className="flex flex-col flex-1 min-h-0 bg-card"
      role="region"
      aria-label="Recent request results"
    >
      {/* Header bar */}
      <div className="flex items-center justify-between px-3 py-1.5 border-b border-border shrink-0">
        <span className="text-xs font-medium text-muted-foreground">
          Request Log
          <span className="ml-1.5 px-1.5 py-0.5 rounded bg-muted text-muted-foreground">
            {recentResults.length}
          </span>
        </span>

        {recentResults.length > 0 && (
          <div className="flex items-center gap-3 text-xs text-muted-foreground pr-1">
            <span>
              {recentResults.filter((r) => !r.error).length} ok
            </span>
            <span className="text-destructive">
              {recentResults.filter((r) => !!r.error).length} err
            </span>
          </div>
        )}
      </div>

      {/* Results list */}
      <div
        id="recent-results-list"
        className="overflow-y-auto flex-1 min-h-0"
        role="table"
        aria-label="Request results log"
      >
        {/* Column headers */}
        <div
          className="flex items-center gap-2 px-3 py-1 text-xs text-muted-foreground uppercase tracking-wider bg-card border-b border-border sticky top-0 z-10"
          role="row"
        >
          <span className="shrink-0 w-3.5" aria-hidden="true" />
          <span className="shrink-0 w-20">Time</span>
          <span className="flex-1 min-w-0">Request</span>
          <span className="shrink-0 w-12 text-right">Status</span>
          <span className="shrink-0 w-16 text-right">Elapsed</span>
        </div>

        {status === "idle" && recentResults.length === 0 ? (
          <div className="flex items-center justify-center py-12 text-sm text-muted-foreground">
            Run a test to see request results here.
          </div>
        ) : recentResults.length === 0 ? (
          <div className="flex items-center justify-center py-12 text-sm text-muted-foreground">
            Waiting for results...
          </div>
        ) : (
          recentResults.map((result, i) => (
            <ResultRow key={`${result.id}-${i}`} result={result} index={i} />
          ))
        )}

        {/* Scroll anchor */}
        <div ref={bottomRef} aria-hidden="true" />
      </div>
    </div>
  );
}
