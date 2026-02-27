import { BarChart3, CheckCircle, XCircle, Clock, Zap, HardDrive, Play, ExternalLink } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/cn";
import { useEngineStore } from "@/stores/useEngineStore";
import { useAppStore } from "@/stores/useAppStore";
import type { TestSummary } from "@/types/results";

// ----------------------------------------------------------------
// Helper formatters
// ----------------------------------------------------------------

function formatDuration(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;
  const msRemainder = ms % 1000;

  if (hours > 0) {
    return `${hours}h ${minutes.toString().padStart(2, "0")}m ${seconds.toString().padStart(2, "0")}s`;
  }
  if (minutes > 0) {
    return `${minutes}m ${seconds.toString().padStart(2, "0")}s`;
  }
  return `${seconds}.${msRemainder.toString().padStart(3, "0")}s`;
}

function formatBytes(bytes: number): string {
  if (bytes >= 1_073_741_824) {
    return `${(bytes / 1_073_741_824).toFixed(2)} GB`;
  }
  if (bytes >= 1_048_576) {
    return `${(bytes / 1_048_576).toFixed(2)} MB`;
  }
  if (bytes >= 1024) {
    return `${(bytes / 1024).toFixed(2)} KB`;
  }
  return `${bytes} B`;
}

function formatDateTime(iso: string): string {
  try {
    return new Date(iso).toLocaleTimeString();
  } catch {
    return iso;
  }
}

function calcDurationMs(summary: TestSummary): number {
  try {
    return new Date(summary.finished_at).getTime() - new Date(summary.started_at).getTime();
  } catch {
    return 0;
  }
}

// ----------------------------------------------------------------
// Metric row
// ----------------------------------------------------------------

interface MetricRowProps {
  label: string;
  value: string;
  highlight?: "success" | "error" | "neutral";
  icon?: React.ReactNode;
}

function MetricRow({ label, value, highlight = "neutral", icon }: MetricRowProps) {
  return (
    <div className="flex items-center justify-between py-1.5 border-b border-border/50 last:border-0">
      <div className="flex items-center gap-2 text-sm text-muted-foreground">
        {icon && <span className="text-muted-foreground" aria-hidden="true">{icon}</span>}
        {label}
      </div>
      <span
        className={cn(
          "text-sm font-medium tabular-nums",
          highlight === "success" && "text-green-600 dark:text-green-400",
          highlight === "error" && "text-destructive",
          highlight === "neutral" && "text-foreground"
        )}
      >
        {value}
      </span>
    </div>
  );
}

// ----------------------------------------------------------------
// Section header
// ----------------------------------------------------------------

function SectionHeader({ title }: { title: string }) {
  return (
    <div className="mt-4 mb-1 first:mt-0">
      <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
        {title}
      </h3>
    </div>
  );
}

// ----------------------------------------------------------------
// TestSummaryPanel
// ----------------------------------------------------------------

interface TestSummaryPanelProps {
  summary: TestSummary;
}

export function TestSummaryPanel({ summary }: TestSummaryPanelProps) {
  const { startTest, reset } = useEngineStore();
  const setActiveView = useAppStore((s) => s.setActiveView);

  const errorRate =
    summary.total_requests > 0
      ? ((summary.failed_requests / summary.total_requests) * 100).toFixed(1)
      : "0.0";

  const durationMs = calcDurationMs(summary);

  function handleRunAgain() {
    reset();
    void startTest(summary.plan_id);
  }

  function handleViewResults() {
    setActiveView("results");
  }

  return (
    <div
      className="flex flex-col bg-card border border-border rounded-lg overflow-hidden"
      role="region"
      aria-label="Test summary"
    >
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-border bg-muted/30">
        <div className="flex items-center gap-2">
          <BarChart3 className="h-4 w-4 text-blue-500" aria-hidden="true" />
          <h2 className="text-sm font-semibold">Test Complete</h2>
          <span className="text-xs text-muted-foreground">— {summary.plan_name}</span>
        </div>
        <div className="flex items-center gap-2">
          <Button
            size="sm"
            variant="outline"
            onClick={handleRunAgain}
            className="h-7 text-xs gap-1.5"
            aria-label="Run test again"
          >
            <Play className="h-3.5 w-3.5" aria-hidden="true" />
            Run Again
          </Button>
          <Button
            size="sm"
            variant="ghost"
            onClick={handleViewResults}
            className="h-7 text-xs gap-1.5"
            aria-label="View full results"
          >
            <ExternalLink className="h-3.5 w-3.5" aria-hidden="true" />
            Full Results
          </Button>
        </div>
      </div>

      {/* Metrics grid */}
      <div className="grid grid-cols-2 gap-x-8 gap-y-0 px-4 py-3">
        {/* Left column */}
        <div>
          <SectionHeader title="Requests" />
          <MetricRow
            label="Total"
            value={summary.total_requests.toLocaleString()}
            icon={<CheckCircle className="h-3.5 w-3.5" />}
          />
          <MetricRow
            label="Successful"
            value={summary.successful_requests.toLocaleString()}
            highlight="success"
            icon={<CheckCircle className="h-3.5 w-3.5" />}
          />
          <MetricRow
            label="Failed"
            value={summary.failed_requests.toLocaleString()}
            highlight={summary.failed_requests > 0 ? "error" : "neutral"}
            icon={<XCircle className="h-3.5 w-3.5" />}
          />
          <MetricRow
            label="Error Rate"
            value={`${errorRate}%`}
            highlight={summary.failed_requests > 0 ? "error" : "success"}
          />

          <SectionHeader title="Throughput" />
          <MetricRow
            label="Requests/sec"
            value={summary.requests_per_second.toFixed(2)}
            icon={<Zap className="h-3.5 w-3.5" />}
          />
          <MetricRow
            label="Total Data"
            value={formatBytes(summary.total_bytes_received)}
            icon={<HardDrive className="h-3.5 w-3.5" />}
          />
        </div>

        {/* Right column */}
        <div>
          <SectionHeader title="Response Time" />
          <MetricRow
            label="Min"
            value={`${summary.min_response_ms} ms`}
            highlight="success"
            icon={<Clock className="h-3.5 w-3.5" />}
          />
          <MetricRow
            label="Mean"
            value={`${Math.round(summary.mean_response_ms)} ms`}
            icon={<Clock className="h-3.5 w-3.5" />}
          />
          <MetricRow
            label="p50"
            value={`${summary.p50_response_ms} ms`}
            icon={<Clock className="h-3.5 w-3.5" />}
          />
          <MetricRow
            label="p95"
            value={`${summary.p95_response_ms} ms`}
            icon={<Clock className="h-3.5 w-3.5" />}
          />
          <MetricRow
            label="p99"
            value={`${summary.p99_response_ms} ms`}
            icon={<Clock className="h-3.5 w-3.5" />}
          />
          <MetricRow
            label="Max"
            value={`${summary.max_response_ms} ms`}
            highlight={summary.max_response_ms > 5000 ? "error" : "neutral"}
            icon={<Clock className="h-3.5 w-3.5" />}
          />

          <SectionHeader title="Duration" />
          <MetricRow
            label="Start"
            value={formatDateTime(summary.started_at)}
          />
          <MetricRow
            label="End"
            value={formatDateTime(summary.finished_at)}
          />
          {durationMs > 0 && (
            <MetricRow
              label="Total"
              value={formatDuration(durationMs)}
            />
          )}
        </div>
      </div>
    </div>
  );
}
