import { useState, useCallback } from "react";
import {
  RefreshCw,
  GitCompare,
  CheckSquare,
  Square,
  Clock,
  Activity,
  Zap,
  AlertTriangle,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/cn";
import type { ResultSummaryEntry } from "@/types/results";

// ----------------------------------------------------------------
// Props
// ----------------------------------------------------------------

export interface ResultHistoryProps {
  entries: ResultSummaryEntry[];
  loading: boolean;
  onRefresh: () => void;
  onSelectRun: (runId: string) => void;
  onCompare: (runIdA: string, runIdB: string) => void;
  currentRunId: string | null;
}

// ----------------------------------------------------------------
// Helpers
// ----------------------------------------------------------------

function formatRelativeTime(iso: string): string {
  try {
    const diff = Date.now() - new Date(iso).getTime();
    const seconds = Math.floor(diff / 1000);
    if (seconds < 60) return `${seconds}s ago`;
    const minutes = Math.floor(seconds / 60);
    if (minutes < 60) return `${minutes} min ago`;
    const hours = Math.floor(minutes / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    return `${days}d ago`;
  } catch {
    return iso;
  }
}

function formatRps(rps: number): string {
  if (rps >= 1000) return `${(rps / 1000).toFixed(1)}k/s`;
  return `${rps.toFixed(1)}/s`;
}

function formatMs(ms: number): string {
  if (ms >= 1000) return `${(ms / 1000).toFixed(2)}s`;
  return `${Math.round(ms)}ms`;
}

function formatErrorRate(rate: number): string {
  return `${(rate * 100).toFixed(1)}%`;
}

// ----------------------------------------------------------------
// HistoryRow
// ----------------------------------------------------------------

interface HistoryRowProps {
  entry: ResultSummaryEntry;
  isSelected: boolean;
  isChecked: boolean;
  onSelect: () => void;
  onToggleCheck: () => void;
  index: number;
}

function HistoryRow({ entry, isSelected, isChecked, onSelect, onToggleCheck, index }: HistoryRowProps) {
  const errorRate = entry.error_rate;
  const hasErrors = errorRate > 0;

  return (
    <div
      className={cn(
        "flex items-center gap-2 px-3 py-2 text-xs border-b border-border/30 cursor-pointer transition-colors",
        "hover:bg-muted/40",
        isSelected && "bg-primary/10 border-l-2 border-l-primary",
        !isSelected && index % 2 === 0 && "bg-background",
        !isSelected && index % 2 !== 0 && "bg-muted/10"
      )}
      role="row"
      aria-selected={isSelected}
      onClick={onSelect}
    >
      {/* Checkbox for comparison selection */}
      <button
        className="shrink-0 text-muted-foreground hover:text-foreground transition-colors"
        onClick={(e) => {
          e.stopPropagation();
          onToggleCheck();
        }}
        aria-label={isChecked ? "Deselect for comparison" : "Select for comparison"}
      >
        {isChecked ? (
          <CheckSquare className="h-3.5 w-3.5 text-primary" aria-hidden="true" />
        ) : (
          <Square className="h-3.5 w-3.5" aria-hidden="true" />
        )}
      </button>

      {/* Plan name */}
      <span
        className={cn(
          "flex-1 min-w-0 truncate font-medium",
          isSelected ? "text-foreground" : "text-foreground/80"
        )}
        title={entry.plan_name}
      >
        {entry.plan_name}
      </span>

      {/* Started at (relative) */}
      <span
        className="shrink-0 text-muted-foreground tabular-nums w-20 text-right"
        title={new Date(entry.started_at).toLocaleString()}
      >
        {formatRelativeTime(entry.started_at)}
      </span>

      {/* Total requests */}
      <span
        className="shrink-0 tabular-nums w-14 text-right text-foreground/70"
        aria-label={`${entry.total_requests} requests`}
        title="Total requests"
      >
        {entry.total_requests.toLocaleString()}
      </span>

      {/* RPS */}
      <span
        className="shrink-0 tabular-nums w-16 text-right text-foreground/70"
        aria-label={`${formatRps(entry.requests_per_second)} requests per second`}
        title="Requests per second"
      >
        {formatRps(entry.requests_per_second)}
      </span>

      {/* Mean response */}
      <span
        className="shrink-0 tabular-nums w-16 text-right text-foreground/70"
        aria-label={`Mean ${formatMs(entry.mean_response_ms)}`}
        title="Mean response time"
      >
        {formatMs(entry.mean_response_ms)}
      </span>

      {/* Error rate */}
      <span
        className={cn(
          "shrink-0 tabular-nums w-14 text-right font-medium",
          hasErrors ? "text-destructive" : "text-green-600 dark:text-green-400"
        )}
        aria-label={`Error rate ${formatErrorRate(errorRate)}`}
        title="Error rate"
      >
        {formatErrorRate(errorRate)}
      </span>
    </div>
  );
}

// ----------------------------------------------------------------
// ResultHistory
// ----------------------------------------------------------------

export function ResultHistory({
  entries,
  loading,
  onRefresh,
  onSelectRun,
  onCompare,
  currentRunId,
}: ResultHistoryProps) {
  const [checkedIds, setCheckedIds] = useState<Set<string>>(new Set());

  const toggleCheck = useCallback((runId: string) => {
    setCheckedIds((prev) => {
      const next = new Set(prev);
      if (next.has(runId)) {
        next.delete(runId);
      } else {
        // Limit to 2 selections
        if (next.size >= 2) {
          const [first] = next;
          next.delete(first);
        }
        next.add(runId);
      }
      return next;
    });
  }, []);

  const checkedArray = Array.from(checkedIds);
  const canCompare = checkedArray.length === 2;

  function handleCompare() {
    if (canCompare) {
      onCompare(checkedArray[0], checkedArray[1]);
    }
  }

  // Sort entries most recent first
  const sorted = [...entries].sort(
    (a, b) => new Date(b.started_at).getTime() - new Date(a.started_at).getTime()
  );

  return (
    <div
      className="flex flex-col flex-1 min-h-0 bg-card"
      role="region"
      aria-label="Test run history"
    >
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-1.5 border-b border-border shrink-0">
        <span className="text-xs font-medium text-muted-foreground">
          History
          {entries.length > 0 && (
            <span className="ml-1.5 px-1.5 py-0.5 rounded bg-muted text-muted-foreground">
              {entries.length}
            </span>
          )}
        </span>

        <div className="flex items-center gap-1">
          {canCompare && (
            <Button
              variant="outline"
              size="sm"
              onClick={handleCompare}
              className="h-6 text-xs gap-1 px-2"
              aria-label="Compare selected runs"
            >
              <GitCompare className="h-3.5 w-3.5" aria-hidden="true" />
              Compare
            </Button>
          )}

          <Button
            variant="ghost"
            size="sm"
            onClick={onRefresh}
            disabled={loading}
            className="h-6 w-6 p-0 text-muted-foreground hover:text-foreground"
            aria-label="Refresh history"
          >
            <RefreshCw
              className={cn("h-3.5 w-3.5", loading && "animate-spin")}
              aria-hidden="true"
            />
          </Button>
        </div>
      </div>

      {/* List */}
      <div
        id="result-history-list"
        className="overflow-y-auto flex-1 min-h-0"
        role="table"
        aria-label="Test run history list"
      >
        {/* Column headers */}
        <div
          className="flex items-center gap-2 px-3 py-1 text-xs text-muted-foreground uppercase tracking-wider bg-card border-b border-border sticky top-0 z-10"
          role="row"
        >
          <span className="shrink-0 w-3.5" aria-hidden="true" />
          <span className="flex-1 min-w-0">Plan</span>
          <span className="shrink-0 w-20 text-right">
            <Clock className="h-3 w-3 inline mr-0.5" aria-hidden="true" />
            When
          </span>
          <span className="shrink-0 w-14 text-right">
            <Activity className="h-3 w-3 inline mr-0.5" aria-hidden="true" />
            Reqs
          </span>
          <span className="shrink-0 w-16 text-right">
            <Zap className="h-3 w-3 inline mr-0.5" aria-hidden="true" />
            RPS
          </span>
          <span className="shrink-0 w-16 text-right">Mean</span>
          <span className="shrink-0 w-14 text-right">
            <AlertTriangle className="h-3 w-3 inline mr-0.5" aria-hidden="true" />
            Err
          </span>
        </div>

        {/* Loading state */}
        {loading && entries.length === 0 && (
          <div className="flex items-center justify-center py-12 text-sm text-muted-foreground">
            Loading history...
          </div>
        )}

        {/* Empty state */}
        {!loading && entries.length === 0 && (
          <div className="flex items-center justify-center py-12 text-sm text-muted-foreground">
            No past runs found.
          </div>
        )}

        {/* Rows */}
        {sorted.map((entry, i) => (
          <HistoryRow
            key={entry.run_id}
            entry={entry}
            index={i}
            isSelected={entry.run_id === currentRunId}
            isChecked={checkedIds.has(entry.run_id)}
            onSelect={() => onSelectRun(entry.run_id)}
            onToggleCheck={() => toggleCheck(entry.run_id)}
          />
        ))}
      </div>
    </div>
  );
}
