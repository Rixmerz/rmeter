import { useEffect, useState } from "react";
import {
  Play,
  Square,
  AlertCircle,
  Activity,
  Clock,
  Zap,
  AlertTriangle,
  Loader2,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/cn";
import { useEngineStore } from "@/stores/useEngineStore";
import { usePlanStore } from "@/stores/usePlanStore";
import type { EngineStatusKind } from "@/types/engine";

// ----------------------------------------------------------------
// Status badge
// ----------------------------------------------------------------

interface StatusBadgeProps {
  status: EngineStatusKind;
}

const STATUS_LABELS: Record<EngineStatusKind, string> = {
  idle: "Idle",
  running: "Running",
  stopping: "Stopping",
  completed: "Completed",
  error: "Error",
};

const STATUS_CLASSES: Record<EngineStatusKind, string> = {
  idle: "bg-muted text-muted-foreground",
  running: "bg-green-500/15 text-green-700 dark:text-green-400 border border-green-500/30",
  stopping: "bg-yellow-500/15 text-yellow-700 dark:text-yellow-400 border border-yellow-500/30",
  completed: "bg-blue-500/15 text-blue-700 dark:text-blue-400 border border-blue-500/30",
  error: "bg-destructive/15 text-destructive border border-destructive/30",
};

function StatusBadge({ status }: StatusBadgeProps) {
  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-xs font-medium",
        STATUS_CLASSES[status]
      )}
      aria-label={`Engine status: ${STATUS_LABELS[status]}`}
    >
      {status === "running" && (
        <span
          className="h-1.5 w-1.5 rounded-full bg-green-500 animate-pulse"
          aria-hidden="true"
        />
      )}
      {status === "stopping" && (
        <Loader2 className="h-3 w-3 animate-spin" aria-hidden="true" />
      )}
      {status === "error" && (
        <AlertCircle className="h-3 w-3" aria-hidden="true" />
      )}
      {STATUS_LABELS[status]}
    </span>
  );
}

// ----------------------------------------------------------------
// Stat item
// ----------------------------------------------------------------

interface StatItemProps {
  icon: React.ReactNode;
  label: string;
  value: string;
}

function StatItem({ icon, label, value }: StatItemProps) {
  return (
    <div
      className="flex items-center gap-1.5 text-xs"
      aria-label={`${label}: ${value}`}
    >
      <span className="text-muted-foreground" aria-hidden="true">
        {icon}
      </span>
      <span className="text-muted-foreground">{label}</span>
      <span className="font-medium tabular-nums">{value}</span>
    </div>
  );
}

// ----------------------------------------------------------------
// Elapsed time formatter
// ----------------------------------------------------------------

function formatElapsed(ms: number): string {
  const totalSeconds = Math.floor(ms / 1000);
  const hours = Math.floor(totalSeconds / 3600);
  const minutes = Math.floor((totalSeconds % 3600) / 60);
  const seconds = totalSeconds % 60;

  if (hours > 0) {
    return `${hours}h ${minutes.toString().padStart(2, "0")}m ${seconds.toString().padStart(2, "0")}s`;
  }
  if (minutes > 0) {
    return `${minutes}m ${seconds.toString().padStart(2, "0")}s`;
  }
  return `${seconds}s`;
}

function formatRps(rps: number): string {
  if (rps >= 1000) {
    return `${(rps / 1000).toFixed(1)}k/s`;
  }
  return `${rps.toFixed(1)}/s`;
}

// ----------------------------------------------------------------
// EngineControlBar
// ----------------------------------------------------------------

export function EngineControlBar() {
  const { status, progress, error, startTest, stopTest, forceStopTest, stoppingAt } =
    useEngineStore();
  const activePlan = usePlanStore((s) => s.activePlan);

  // Track how long we've been in "stopping" state
  const [showForceStop, setShowForceStop] = useState(false);

  useEffect(() => {
    if (status !== "stopping" || stoppingAt === null) {
      setShowForceStop(false);
      return;
    }

    const elapsed = Date.now() - stoppingAt;
    const remaining = 3000 - elapsed;

    if (remaining <= 0) {
      setShowForceStop(true);
      return;
    }

    const timer = setTimeout(() => setShowForceStop(true), remaining);
    return () => clearTimeout(timer);
  }, [status, stoppingAt]);

  const isRunning = status === "running";
  const isStopping = status === "stopping";
  const isActive = isRunning || isStopping;

  function handleRunTest() {
    if (!activePlan) return;
    void startTest(activePlan.id);
  }

  function handleStop() {
    void stopTest();
  }

  function handleForceStop() {
    void forceStopTest();
  }

  return (
    <div
      className={cn(
        "flex items-center gap-3 px-4 py-2 border-b border-border bg-card shrink-0",
        "flex-wrap"
      )}
      role="region"
      aria-label="Engine controls"
    >
      {/* Run button */}
      <Button
        size="sm"
        onClick={handleRunTest}
        disabled={isActive || !activePlan}
        className={cn(
          "h-7 text-xs gap-1.5 shrink-0",
          !isActive && activePlan
            ? "bg-green-600 hover:bg-green-700 text-white"
            : ""
        )}
        aria-label="Run test"
        title={!activePlan ? "Select a test plan first" : "Run test"}
      >
        <Play className="h-3.5 w-3.5" aria-hidden="true" />
        Run Test
      </Button>

      {/* Stop button — visible when running or stopping */}
      {isActive && (
        <Button
          size="sm"
          variant="destructive"
          onClick={handleStop}
          disabled={isStopping && !showForceStop}
          className="h-7 text-xs gap-1.5 shrink-0"
          aria-label="Stop test"
        >
          <Square className="h-3.5 w-3.5" aria-hidden="true" />
          Stop
        </Button>
      )}

      {/* Force Stop — only visible when stuck in stopping for > 3s */}
      {showForceStop && (
        <Button
          size="sm"
          variant="outline"
          onClick={handleForceStop}
          className="h-7 text-xs gap-1.5 shrink-0 border-destructive text-destructive hover:bg-destructive/10"
          aria-label="Force stop test"
          title="Force stop (kills immediately)"
        >
          <AlertTriangle className="h-3.5 w-3.5" aria-hidden="true" />
          Force Stop
        </Button>
      )}

      {/* Divider */}
      <div className="h-5 w-px bg-border shrink-0" aria-hidden="true" />

      {/* Status badge */}
      <StatusBadge status={status} />

      {/* Progress stats — only when we have progress data */}
      {progress && (
        <>
          <div className="h-5 w-px bg-border shrink-0" aria-hidden="true" />
          <div className="flex items-center gap-4 flex-wrap">
            <StatItem
              icon={<Activity className="h-3.5 w-3.5" />}
              label="Reqs"
              value={progress.completed_requests.toLocaleString()}
            />
            {progress.total_errors > 0 && (
              <StatItem
                icon={<AlertCircle className="h-3.5 w-3.5 text-destructive" />}
                label="Errors"
                value={progress.total_errors.toLocaleString()}
              />
            )}
            <StatItem
              icon={<Clock className="h-3.5 w-3.5" />}
              label="Elapsed"
              value={formatElapsed(progress.elapsed_ms)}
            />
            <StatItem
              icon={<Zap className="h-3.5 w-3.5" />}
              label="RPS"
              value={formatRps(progress.current_rps)}
            />
            {progress.active_threads > 0 && (
              <StatItem
                icon={<Activity className="h-3.5 w-3.5" />}
                label="Threads"
                value={String(progress.active_threads)}
              />
            )}
          </div>
        </>
      )}

      {/* Error message */}
      {error && (
        <>
          <div className="h-5 w-px bg-border shrink-0" aria-hidden="true" />
          <div
            className="flex items-center gap-1.5 text-xs text-destructive"
            role="alert"
          >
            <AlertCircle className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
            <span className="truncate max-w-xs">{error}</span>
          </div>
        </>
      )}

      {/* Plan name indicator */}
      {activePlan && (
        <div className="ml-auto text-xs text-muted-foreground truncate max-w-48 shrink-0">
          {activePlan.name}
        </div>
      )}
    </div>
  );
}
