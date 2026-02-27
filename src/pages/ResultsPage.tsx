import { useState, useEffect, useCallback } from "react";
import { BarChart3 } from "lucide-react";
import { cn } from "@/lib/cn";
import { useEngineStore } from "@/stores/useEngineStore";
import { TestSummaryPanel } from "@/components/engine/TestSummaryPanel";
import { RecentResultsLog } from "@/components/engine/RecentResultsLog";
import { ExportBar } from "@/components/engine/ExportBar";
import { ResultHistory } from "@/components/engine/ResultHistory";
import { ComparisonDialog } from "@/components/engine/ComparisonDialog";
import { LiveStatsBar } from "@/components/dashboard/LiveStatsBar";
import { ResponseTimeChart } from "@/components/dashboard/ResponseTimeChart";
import { ThroughputChart } from "@/components/dashboard/ThroughputChart";
import { ErrorRateChart } from "@/components/dashboard/ErrorRateChart";
import { ActiveThreadsChart } from "@/components/dashboard/ActiveThreadsChart";
import { RequestInspector } from "@/components/engine/RequestInspector";
import { listResults, compareResults } from "@/lib/commands";
import type { ResultSummaryEntry, ComparisonResult } from "@/types/results";

// ----------------------------------------------------------------
// Top-level section tabs
// ----------------------------------------------------------------

type SectionTab = "dashboard" | "inspector" | "log" | "history";

const SECTION_TABS: Array<{ id: SectionTab; label: string }> = [
  { id: "dashboard", label: "Dashboard" },
  { id: "inspector", label: "Inspector" },
  { id: "log", label: "Request Log" },
  { id: "history", label: "History" },
];

// ----------------------------------------------------------------
// Chart sub-tabs (within dashboard)
// ----------------------------------------------------------------

type ChartTab = "response-time" | "throughput" | "error-rate" | "threads";

const CHART_TABS: Array<{ id: ChartTab; label: string }> = [
  { id: "response-time", label: "Response Time" },
  { id: "throughput", label: "Throughput" },
  { id: "error-rate", label: "Error Rate" },
  { id: "threads", label: "Active Threads" },
];

// ----------------------------------------------------------------
// ResultsPage
// ----------------------------------------------------------------

export function ResultsPage() {
  const lastSummary = useEngineStore((s) => s.lastSummary);
  const status = useEngineStore((s) => s.status);
  const chartData = useEngineStore((s) => s.chartData);

  const [sectionTab, setSectionTab] = useState<SectionTab>("dashboard");
  const [chartTab, setChartTab] = useState<ChartTab>("response-time");

  // ---- Export & History state ----
  const [currentRunId, setCurrentRunId] = useState<string | null>(null);
  const [resultHistory, setResultHistory] = useState<ResultSummaryEntry[]>([]);
  const [historyLoading, setHistoryLoading] = useState(false);
  const [comparison, setComparison] = useState<ComparisonResult | null>(null);
  const [comparisonOpen, setComparisonOpen] = useState(false);
  const [compareError, setCompareError] = useState<string | null>(null);

  // Auto-switch to dashboard when a test starts
  useEffect(() => {
    if (status === "running") {
      setSectionTab("dashboard");
    }
  }, [status]);

  // Load history from backend
  const refreshHistory = useCallback(async () => {
    setHistoryLoading(true);
    try {
      const entries = await listResults();
      setResultHistory(entries);
      if (entries.length > 0 && status === "completed") {
        const sorted = [...entries].sort(
          (a, b) => new Date(b.started_at).getTime() - new Date(a.started_at).getTime()
        );
        setCurrentRunId((prev) => prev ?? sorted[0].run_id);
      }
    } catch {
      // Backend may not have results yet
    } finally {
      setHistoryLoading(false);
    }
  }, [status]);

  useEffect(() => {
    void refreshHistory();
  }, [status, refreshHistory]);

  useEffect(() => {
    if (status === "completed" && resultHistory.length > 0) {
      const sorted = [...resultHistory].sort(
        (a, b) => new Date(b.started_at).getTime() - new Date(a.started_at).getTime()
      );
      setCurrentRunId(sorted[0].run_id);
    }
  }, [status, resultHistory]);

  // ---- Handlers ----
  function handleSelectRun(runId: string) {
    setCurrentRunId(runId);
  }

  async function handleCompare(runIdA: string, runIdB: string) {
    setCompareError(null);
    try {
      const result = await compareResults(runIdA, runIdB);
      setComparison(result);
      setComparisonOpen(true);
    } catch (err) {
      setCompareError(err instanceof Error ? err.message : String(err));
    }
  }

  // ---- Derived ----
  const showDashboard =
    status === "running" ||
    status === "stopping" ||
    status === "completed" ||
    chartData.length > 0;

  const exportRunId = currentRunId ?? null;
  const exportPlanName =
    currentRunId !== null
      ? (resultHistory.find((e) => e.run_id === currentRunId)?.plan_name ??
          lastSummary?.plan_name ?? "results")
      : (lastSummary?.plan_name ?? "results");

  // ---- Empty state ----
  if (!showDashboard && resultHistory.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-full text-center gap-4">
        <div className="rounded-full bg-muted p-6">
          <BarChart3 className="h-10 w-10 text-muted-foreground" aria-hidden="true" />
        </div>
        <div>
          <h1 className="text-2xl font-bold tracking-tight">Results</h1>
          <p className="text-muted-foreground mt-2">
            No test results yet. Run a test plan to see results here.
          </p>
        </div>
      </div>
    );
  }

  // ---- Main view ----
  return (
    <div className="flex flex-col h-full overflow-hidden">
      {/* Compare error banner */}
      {compareError && (
        <div
          className="px-4 py-2 text-xs text-destructive bg-destructive/10 border-b border-destructive/30 shrink-0"
          role="alert"
        >
          Compare failed: {compareError}
        </div>
      )}

      {/* Section tab bar */}
      <div className="flex items-center gap-1 px-4 border-b border-border bg-card shrink-0" role="tablist" aria-label="Results sections">
        {SECTION_TABS.map((tab) => (
          <button
            key={tab.id}
            role="tab"
            aria-selected={sectionTab === tab.id}
            onClick={() => setSectionTab(tab.id)}
            className={cn(
              "px-4 py-2.5 text-sm font-medium transition-colors border-b-2 -mb-px",
              sectionTab === tab.id
                ? "border-primary text-foreground"
                : "border-transparent text-muted-foreground hover:text-foreground hover:border-border"
            )}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Tab content */}
      <div className="flex-1 min-h-0 overflow-hidden flex flex-col">
        {/* ============ DASHBOARD TAB ============ */}
        {sectionTab === "dashboard" && (
          <div className="flex-1 min-h-0 flex flex-col overflow-auto">
            {/* Summary panel */}
            {lastSummary && (
              <div className="px-4 pt-4 pb-2 shrink-0">
                <TestSummaryPanel summary={lastSummary} />
              </div>
            )}

            {/* Live stats bar */}
            {(status === "running" || status === "stopping" || chartData.length > 0) && (
              <LiveStatsBar />
            )}

            {/* Chart sub-tabs */}
            {chartData.length > 0 && (
              <div className="flex-1 min-h-0 flex flex-col px-4 pt-3 pb-4 gap-3">
                <div className="flex gap-1 border-b border-border shrink-0" role="tablist" aria-label="Chart views">
                  {CHART_TABS.map((tab) => (
                    <button
                      key={tab.id}
                      role="tab"
                      aria-selected={chartTab === tab.id}
                      onClick={() => setChartTab(tab.id)}
                      className={cn(
                        "px-3 py-1.5 text-xs font-medium transition-colors border-b-2 -mb-px",
                        chartTab === tab.id
                          ? "border-primary text-foreground"
                          : "border-transparent text-muted-foreground hover:text-foreground hover:border-border"
                      )}
                    >
                      {tab.label}
                    </button>
                  ))}
                </div>

                <div className="flex-1 min-h-[300px]" role="tabpanel" aria-label={CHART_TABS.find((t) => t.id === chartTab)?.label}>
                  {chartTab === "response-time" && <ResponseTimeChart />}
                  {chartTab === "throughput" && <ThroughputChart />}
                  {chartTab === "error-rate" && <ErrorRateChart />}
                  {chartTab === "threads" && <ActiveThreadsChart />}
                </div>
              </div>
            )}

            {/* Empty chart state */}
            {chartData.length === 0 && !lastSummary && (
              <div className="flex-1 flex items-center justify-center text-muted-foreground text-sm">
                Charts will appear here when a test is running.
              </div>
            )}
          </div>
        )}

        {/* ============ INSPECTOR TAB ============ */}
        {sectionTab === "inspector" && (
          <div className="flex-1 min-h-0 flex flex-col">
            <RequestInspector />
          </div>
        )}

        {/* ============ REQUEST LOG TAB ============ */}
        {sectionTab === "log" && (
          <div className="flex-1 min-h-0 flex flex-col">
            <RecentResultsLog />
          </div>
        )}

        {/* ============ HISTORY TAB ============ */}
        {sectionTab === "history" && (
          <div className="flex-1 min-h-0 flex flex-col overflow-auto">
            {/* Export bar */}
            {exportRunId !== null && (
              <ExportBar runId={exportRunId} planName={exportPlanName} />
            )}

            {/* History list - full height */}
            <div className="flex-1 min-h-0">
              <ResultHistory
                entries={resultHistory}
                loading={historyLoading}
                onRefresh={() => void refreshHistory()}
                onSelectRun={handleSelectRun}
                onCompare={(a, b) => void handleCompare(a, b)}
                currentRunId={currentRunId}
              />
            </div>
          </div>
        )}
      </div>

      {/* Comparison dialog */}
      {comparison !== null && (
        <ComparisonDialog
          comparison={comparison}
          open={comparisonOpen}
          onClose={() => setComparisonOpen(false)}
        />
      )}
    </div>
  );
}
