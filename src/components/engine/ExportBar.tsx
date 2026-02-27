import { useState } from "react";
import { Download, FileText, FileJson, Globe, CheckCircle, AlertCircle } from "lucide-react";
import { save } from "@tauri-apps/plugin-dialog";
import { writeTextFile } from "@tauri-apps/plugin-fs";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/cn";
import { exportResultsCsv, exportResultsJson, exportResultsHtml } from "@/lib/commands";

// ----------------------------------------------------------------
// Types
// ----------------------------------------------------------------

export interface ExportBarProps {
  runId: string;
  planName: string;
}

type ExportStatus = "idle" | "loading" | "success" | "error";

interface ExportState {
  csv: ExportStatus;
  json: ExportStatus;
  html: ExportStatus;
}

// ----------------------------------------------------------------
// Helpers
// ----------------------------------------------------------------

function makeTimestamp(): string {
  const now = new Date();
  return [
    now.getFullYear(),
    String(now.getMonth() + 1).padStart(2, "0"),
    String(now.getDate()).padStart(2, "0"),
    "_",
    String(now.getHours()).padStart(2, "0"),
    String(now.getMinutes()).padStart(2, "0"),
    String(now.getSeconds()).padStart(2, "0"),
  ].join("");
}

function sanitizeName(name: string): string {
  return name.replace(/[^a-zA-Z0-9_-]/g, "_").slice(0, 40);
}

// ----------------------------------------------------------------
// ExportBar
// ----------------------------------------------------------------

export function ExportBar({ runId, planName }: ExportBarProps) {
  const [state, setState] = useState<ExportState>({
    csv: "idle",
    json: "idle",
    html: "idle",
  });

  const [lastError, setLastError] = useState<string | null>(null);

  function setStatus(key: keyof ExportState, status: ExportStatus) {
    setState((s) => ({ ...s, [key]: status }));
  }

  async function handleExport(
    format: keyof ExportState,
    fetcher: (id: string) => Promise<string>,
    ext: string,
    mimeType: string
  ) {
    if (state[format] === "loading") return;

    setStatus(format, "loading");
    setLastError(null);

    try {
      const content = await fetcher(runId);

      const ts = makeTimestamp();
      const safe = sanitizeName(planName);
      const defaultName = `${safe}_${ts}.${ext}`;

      const filePath = await save({
        title: `Export results as ${ext.toUpperCase()}`,
        defaultPath: defaultName,
        filters: [
          {
            name: mimeType,
            extensions: [ext],
          },
        ],
      });

      if (filePath === null) {
        // User cancelled
        setStatus(format, "idle");
        return;
      }

      await writeTextFile(filePath, content);
      setStatus(format, "success");

      // Reset success indicator after 2.5 seconds
      setTimeout(() => setStatus(format, "idle"), 2500);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setLastError(message);
      setStatus(format, "error");
      setTimeout(() => setStatus(format, "idle"), 3000);
    }
  }

  const isAnyLoading = Object.values(state).some((s) => s === "loading");

  return (
    <div
      className="flex items-center gap-2 px-4 py-2 border-b border-border bg-muted/10 shrink-0 flex-wrap"
      role="toolbar"
      aria-label="Export results"
    >
      {/* Label */}
      <div className="flex items-center gap-1.5 text-xs text-muted-foreground mr-1">
        <Download className="h-3.5 w-3.5" aria-hidden="true" />
        <span className="font-medium">Export</span>
      </div>

      {/* CSV */}
      <ExportButton
        label="CSV"
        icon={<FileText className="h-3.5 w-3.5" aria-hidden="true" />}
        status={state.csv}
        disabled={isAnyLoading}
        onClick={() => handleExport("csv", exportResultsCsv, "csv", "CSV")}
        aria-label="Export results as CSV"
      />

      {/* JSON */}
      <ExportButton
        label="JSON"
        icon={<FileJson className="h-3.5 w-3.5" aria-hidden="true" />}
        status={state.json}
        disabled={isAnyLoading}
        onClick={() => handleExport("json", exportResultsJson, "json", "JSON")}
        aria-label="Export results as JSON"
      />

      {/* HTML Report */}
      <ExportButton
        label="HTML Report"
        icon={<Globe className="h-3.5 w-3.5" aria-hidden="true" />}
        status={state.html}
        disabled={isAnyLoading}
        onClick={() => handleExport("html", exportResultsHtml, "html", "HTML")}
        aria-label="Export results as HTML report"
      />

      {/* Error message */}
      {lastError && (
        <div
          className="flex items-center gap-1.5 text-xs text-destructive ml-2"
          role="alert"
          aria-live="assertive"
        >
          <AlertCircle className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
          <span className="truncate max-w-64" title={lastError}>
            {lastError}
          </span>
        </div>
      )}
    </div>
  );
}

// ----------------------------------------------------------------
// ExportButton
// ----------------------------------------------------------------

interface ExportButtonProps {
  label: string;
  icon: React.ReactNode;
  status: ExportStatus;
  disabled: boolean;
  onClick: () => void;
  "aria-label": string;
}

function ExportButton({ label, icon, status, disabled, onClick, "aria-label": ariaLabel }: ExportButtonProps) {
  const isLoading = status === "loading";
  const isSuccess = status === "success";
  const isError = status === "error";

  return (
    <Button
      size="sm"
      variant="outline"
      onClick={onClick}
      disabled={disabled}
      aria-label={ariaLabel}
      aria-busy={isLoading}
      className={cn(
        "h-7 text-xs gap-1.5 transition-all",
        isSuccess && "border-green-500 text-green-600 dark:text-green-400 bg-green-500/10",
        isError && "border-destructive text-destructive bg-destructive/10"
      )}
    >
      {isSuccess ? (
        <CheckCircle className="h-3.5 w-3.5" aria-hidden="true" />
      ) : isError ? (
        <AlertCircle className="h-3.5 w-3.5" aria-hidden="true" />
      ) : (
        icon
      )}
      {isLoading ? "Saving..." : isSuccess ? "Saved!" : isError ? "Failed" : label}
    </Button>
  );
}
