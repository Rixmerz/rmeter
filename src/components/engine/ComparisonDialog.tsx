import { X, TrendingUp, TrendingDown, Minus } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/cn";
import type { ComparisonResult, TestSummary } from "@/types/results";

// ----------------------------------------------------------------
// Props
// ----------------------------------------------------------------

export interface ComparisonDialogProps {
  comparison: ComparisonResult;
  open: boolean;
  onClose: () => void;
}

// ----------------------------------------------------------------
// Helpers
// ----------------------------------------------------------------

function formatMs(ms: number): string {
  if (ms >= 1000) return `${(ms / 1000).toFixed(2)}s`;
  return `${Math.round(ms)} ms`;
}

function formatRps(rps: number): string {
  return `${rps.toFixed(2)}/s`;
}

function formatErrorRate(rate: number): string {
  // error_rate is already a fraction (0–1) in ComparisonResult.run_a / run_b
  // The delta_error_rate is a raw fraction delta
  return `${(rate * 100).toFixed(2)}%`;
}

function formatDeltaMs(delta: number): string {
  const sign = delta > 0 ? "+" : "";
  return `${sign}${Math.round(delta)} ms`;
}

function formatDeltaRps(delta: number): string {
  const sign = delta > 0 ? "+" : "";
  return `${sign}${delta.toFixed(2)}/s`;
}

function formatDeltaErrorRate(delta: number): string {
  const sign = delta > 0 ? "+" : "";
  return `${sign}${(delta * 100).toFixed(2)}%`;
}

function formatDeltaRequests(delta: number): string {
  const sign = delta > 0 ? "+" : "";
  return `${sign}${delta.toLocaleString()}`;
}

type DeltaDirection = "better" | "worse" | "neutral";

/** Determine if a delta represents improvement or regression */
function direction(delta: number, lowerIsBetter: boolean): DeltaDirection {
  if (Math.abs(delta) < 0.0001) return "neutral";
  if (lowerIsBetter) {
    return delta < 0 ? "better" : "worse";
  } else {
    return delta > 0 ? "better" : "worse";
  }
}

// ----------------------------------------------------------------
// DeltaCell
// ----------------------------------------------------------------

interface DeltaCellProps {
  delta: number;
  lowerIsBetter: boolean;
  format: (v: number) => string;
}

function DeltaCell({ delta, lowerIsBetter, format }: DeltaCellProps) {
  const dir = direction(delta, lowerIsBetter);

  return (
    <td
      className={cn(
        "px-3 py-2.5 text-sm tabular-nums text-right font-medium",
        dir === "better" && "text-green-600 dark:text-green-400",
        dir === "worse" && "text-destructive",
        dir === "neutral" && "text-muted-foreground"
      )}
      aria-label={`Delta: ${format(delta)}`}
    >
      <span className="inline-flex items-center gap-1 justify-end">
        {dir === "better" && (
          <TrendingDown className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
        )}
        {dir === "worse" && (
          <TrendingUp className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
        )}
        {dir === "neutral" && (
          <Minus className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
        )}
        {format(delta)}
      </span>
    </td>
  );
}

// ----------------------------------------------------------------
// Metric row definition
// ----------------------------------------------------------------

interface MetricDef {
  label: string;
  getA: (s: TestSummary) => number;
  getB: (s: TestSummary) => number;
  delta: number;
  formatValue: (v: number) => string;
  formatDelta: (v: number) => string;
  lowerIsBetter: boolean;
}

function buildMetrics(c: ComparisonResult): MetricDef[] {
  // Error rate: compute from requests for display
  const errorRateA =
    c.run_a.total_requests > 0
      ? c.run_a.failed_requests / c.run_a.total_requests
      : 0;
  const errorRateB =
    c.run_b.total_requests > 0
      ? c.run_b.failed_requests / c.run_b.total_requests
      : 0;

  return [
    {
      label: "Total Requests",
      getA: (s) => s.total_requests,
      getB: (s) => s.total_requests,
      delta: c.delta_total_requests,
      formatValue: (v) => v.toLocaleString(),
      formatDelta: formatDeltaRequests,
      lowerIsBetter: false,
    },
    {
      label: "Mean Response",
      getA: (s) => s.mean_response_ms,
      getB: (s) => s.mean_response_ms,
      delta: c.delta_mean_ms,
      formatValue: formatMs,
      formatDelta: formatDeltaMs,
      lowerIsBetter: true,
    },
    {
      label: "p95 Response",
      getA: (s) => s.p95_response_ms,
      getB: (s) => s.p95_response_ms,
      delta: c.delta_p95_ms,
      formatValue: formatMs,
      formatDelta: formatDeltaMs,
      lowerIsBetter: true,
    },
    {
      label: "p99 Response",
      getA: (s) => s.p99_response_ms,
      getB: (s) => s.p99_response_ms,
      delta: c.delta_p99_ms,
      formatValue: formatMs,
      formatDelta: formatDeltaMs,
      lowerIsBetter: true,
    },
    {
      label: "Requests/sec",
      getA: (s) => s.requests_per_second,
      getB: (s) => s.requests_per_second,
      delta: c.delta_rps,
      formatValue: formatRps,
      formatDelta: formatDeltaRps,
      lowerIsBetter: false,
    },
    {
      label: "Error Rate",
      getA: () => errorRateA,
      getB: () => errorRateB,
      delta: c.delta_error_rate,
      formatValue: formatErrorRate,
      formatDelta: formatDeltaErrorRate,
      lowerIsBetter: true,
    },
  ];
}

// ----------------------------------------------------------------
// ComparisonDialog
// ----------------------------------------------------------------

export function ComparisonDialog({ comparison, open, onClose }: ComparisonDialogProps) {
  if (!open) return null;

  const metrics = buildMetrics(comparison);

  function formatRunLabel(s: TestSummary): string {
    try {
      return `${s.plan_name} — ${new Date(s.started_at).toLocaleString()}`;
    } catch {
      return s.plan_name;
    }
  }

  return (
    /* Backdrop */
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/60"
      role="dialog"
      aria-modal="true"
      aria-label="Compare test runs"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      {/* Panel */}
      <div
        className="relative flex flex-col bg-background border border-border rounded-xl shadow-2xl w-full max-w-2xl mx-4 overflow-hidden"
        style={{ maxHeight: "85vh" }}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-border bg-muted/20 shrink-0">
          <div>
            <h2 className="text-base font-semibold">Run Comparison</h2>
            <p className="text-xs text-muted-foreground mt-0.5">
              Side-by-side metric comparison between two test runs
            </p>
          </div>
          <Button
            variant="ghost"
            size="icon"
            onClick={onClose}
            className="h-8 w-8 shrink-0"
            aria-label="Close comparison"
          >
            <X className="h-4 w-4" aria-hidden="true" />
          </Button>
        </div>

        {/* Table */}
        <div className="overflow-auto flex-1">
          <table className="w-full text-sm border-collapse">
            <thead>
              <tr className="bg-muted/30 sticky top-0 z-10">
                <th
                  scope="col"
                  className="px-3 py-2.5 text-left text-xs font-semibold uppercase tracking-wider text-muted-foreground border-b border-border w-32"
                >
                  Metric
                </th>
                <th
                  scope="col"
                  className="px-3 py-2.5 text-right text-xs font-semibold uppercase tracking-wider text-muted-foreground border-b border-border"
                >
                  <span className="block truncate max-w-48" title={formatRunLabel(comparison.run_a)}>
                    Run A
                  </span>
                  <span
                    className="block text-[10px] font-normal text-muted-foreground/70 truncate max-w-48"
                    title={formatRunLabel(comparison.run_a)}
                  >
                    {comparison.run_a.plan_name}
                  </span>
                </th>
                <th
                  scope="col"
                  className="px-3 py-2.5 text-right text-xs font-semibold uppercase tracking-wider text-muted-foreground border-b border-border"
                >
                  <span className="block truncate max-w-48" title={formatRunLabel(comparison.run_b)}>
                    Run B
                  </span>
                  <span
                    className="block text-[10px] font-normal text-muted-foreground/70 truncate max-w-48"
                    title={formatRunLabel(comparison.run_b)}
                  >
                    {comparison.run_b.plan_name}
                  </span>
                </th>
                <th
                  scope="col"
                  className="px-3 py-2.5 text-right text-xs font-semibold uppercase tracking-wider text-muted-foreground border-b border-border"
                >
                  Delta (B vs A)
                </th>
              </tr>
            </thead>
            <tbody>
              {metrics.map((m, i) => (
                <tr
                  key={m.label}
                  className={cn(
                    "border-b border-border/30 transition-colors hover:bg-muted/20",
                    i % 2 === 0 ? "bg-background" : "bg-muted/10"
                  )}
                >
                  <th
                    scope="row"
                    className="px-3 py-2.5 text-left text-xs font-medium text-muted-foreground"
                  >
                    {m.label}
                  </th>
                  <td className="px-3 py-2.5 text-right tabular-nums text-sm">
                    {m.formatValue(m.getA(comparison.run_a))}
                  </td>
                  <td className="px-3 py-2.5 text-right tabular-nums text-sm">
                    {m.formatValue(m.getB(comparison.run_b))}
                  </td>
                  <DeltaCell
                    delta={m.delta}
                    lowerIsBetter={m.lowerIsBetter}
                    format={m.formatDelta}
                  />
                </tr>
              ))}
            </tbody>
          </table>
        </div>

        {/* Legend */}
        <div className="flex items-center gap-4 px-5 py-3 border-t border-border bg-muted/10 shrink-0 text-xs text-muted-foreground">
          <div className="flex items-center gap-1">
            <TrendingDown className="h-3.5 w-3.5 text-green-600 dark:text-green-400" aria-hidden="true" />
            <span>Improvement</span>
          </div>
          <div className="flex items-center gap-1">
            <TrendingUp className="h-3.5 w-3.5 text-destructive" aria-hidden="true" />
            <span>Regression</span>
          </div>
          <div className="flex items-center gap-1">
            <Minus className="h-3.5 w-3.5" aria-hidden="true" />
            <span>No change</span>
          </div>
          <span className="ml-auto text-muted-foreground/60">
            Delta = Run B minus Run A
          </span>
        </div>
      </div>
    </div>
  );
}
