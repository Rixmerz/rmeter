import { Activity, Clock, Zap, AlertTriangle, Users, HardDrive } from "lucide-react";
import { cn } from "@/lib/cn";
import { useEngineStore } from "@/stores/useEngineStore";

// ----------------------------------------------------------------
// StatCard
// ----------------------------------------------------------------

interface StatCardProps {
  icon: React.ReactNode;
  label: string;
  value: string;
  className?: string;
  valueClassName?: string;
}

function StatCard({ icon, label, value, className, valueClassName }: StatCardProps) {
  return (
    <div
      className={cn(
        "flex flex-col items-center gap-1 px-4 py-3 rounded-lg border border-border bg-card min-w-[100px]",
        className
      )}
    >
      <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
        {icon}
        <span>{label}</span>
      </div>
      <span className={cn("text-lg font-semibold tabular-nums", valueClassName)}>
        {value}
      </span>
    </div>
  );
}

// ----------------------------------------------------------------
// LiveStatsBar
// ----------------------------------------------------------------

export function LiveStatsBar() {
  const progress = useEngineStore((s) => s.progress);

  const totalRequests = progress?.completed_requests ?? 0;
  const totalErrors = progress?.total_errors ?? 0;
  const activeThr = progress?.active_threads ?? 0;
  const rps = progress?.current_rps ?? 0;
  const meanMs = progress?.mean_ms ?? 0;
  const p95Ms = progress?.p95_ms ?? 0;

  const errorRate =
    totalRequests > 0
      ? ((totalErrors / totalRequests) * 100).toFixed(1)
      : "0.0";

  const hasErrors = totalErrors > 0;

  return (
    <div
      className="flex items-center gap-2 px-4 py-2 border-b border-border bg-muted/20 overflow-x-auto shrink-0"
      role="region"
      aria-label="Live test statistics"
    >
      <StatCard
        icon={<Activity className="h-3.5 w-3.5" aria-hidden="true" />}
        label="Requests"
        value={totalRequests.toLocaleString()}
      />
      <StatCard
        icon={<AlertTriangle className="h-3.5 w-3.5" aria-hidden="true" />}
        label="Error Rate"
        value={`${errorRate}%`}
        valueClassName={hasErrors ? "text-destructive" : "text-green-600 dark:text-green-400"}
      />
      <StatCard
        icon={<Clock className="h-3.5 w-3.5" aria-hidden="true" />}
        label="Mean"
        value={`${Math.round(meanMs)} ms`}
      />
      <StatCard
        icon={<Clock className="h-3.5 w-3.5" aria-hidden="true" />}
        label="p95"
        value={`${Math.round(p95Ms)} ms`}
      />
      <StatCard
        icon={<Zap className="h-3.5 w-3.5" aria-hidden="true" />}
        label="RPS"
        value={rps.toFixed(1)}
      />
      <StatCard
        icon={<Users className="h-3.5 w-3.5" aria-hidden="true" />}
        label="Threads"
        value={activeThr.toString()}
      />
      {progress && (
        <StatCard
          icon={<HardDrive className="h-3.5 w-3.5" aria-hidden="true" />}
          label="Errors"
          value={totalErrors.toLocaleString()}
          valueClassName={hasErrors ? "text-destructive" : undefined}
        />
      )}
    </div>
  );
}
