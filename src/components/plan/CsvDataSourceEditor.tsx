import { useState } from "react";
import { Plus, Trash2, FileSpreadsheet, Upload } from "lucide-react";
import { cn } from "@/lib/cn";
import type { CsvDataSource, CsvSharingMode } from "@/types/plan";
import {
  addCsvDataSource,
  removeCsvDataSource,
  updateCsvDataSource,
} from "@/lib/commands";

// ----------------------------------------------------------------
// Shared style
// ----------------------------------------------------------------

const inputClass = cn(
  "w-full text-xs px-2 py-1 rounded border border-input bg-background",
  "focus:outline-none focus:ring-1 focus:ring-ring"
);

// ----------------------------------------------------------------
// AddCsvForm — paste or load CSV content
// ----------------------------------------------------------------

interface AddCsvFormProps {
  planId: string;
  onAdded: () => void;
  onCancel: () => void;
}

function AddCsvForm({ planId, onAdded, onCancel }: AddCsvFormProps) {
  const [name, setName] = useState("CSV Data");
  const [csvText, setCsvText] = useState("");
  const [delimiter, setDelimiter] = useState(",");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSave() {
    const trimmed = csvText.trim();
    if (!trimmed) {
      setError("CSV content is required");
      return;
    }
    if (!name.trim()) {
      setError("Name is required");
      return;
    }
    setSaving(true);
    setError(null);
    try {
      await addCsvDataSource(planId, name.trim(), trimmed, delimiter || ",");
      onAdded();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setSaving(false);
    }
  }

  async function handleFileLoad() {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = ".csv,.tsv,.txt";
    input.onchange = async () => {
      const file = input.files?.[0];
      if (!file) return;
      const text = await file.text();
      setCsvText(text);
      if (name === "CSV Data") {
        setName(file.name.replace(/\.\w+$/, ""));
      }
      // Auto-detect delimiter
      const firstLine = text.split("\n")[0] ?? "";
      if (firstLine.includes("\t") && !firstLine.includes(",")) {
        setDelimiter("\t");
      } else if (firstLine.includes(";") && !firstLine.includes(",")) {
        setDelimiter(";");
      }
    };
    input.click();
  }

  // Preview: parse first few rows
  const previewLines = csvText.trim().split("\n").slice(0, 6);
  const previewDelim = delimiter === "\t" ? "\t" : delimiter || ",";
  const previewCols = previewLines[0]?.split(previewDelim).map((c) => c.trim()) ?? [];
  const previewRows = previewLines.slice(1).map((l) => l.split(previewDelim).map((c) => c.trim()));

  return (
    <div className="border border-border rounded p-3 space-y-3 bg-muted/20">
      <div className="flex items-center gap-2">
        <div className="flex-1 space-y-1">
          <label className="text-xs text-muted-foreground">Name</label>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="My CSV Data"
            className={inputClass}
            aria-label="Data source name"
          />
        </div>
        <div className="w-20 space-y-1">
          <label className="text-xs text-muted-foreground">Delimiter</label>
          <select
            value={delimiter}
            onChange={(e) => setDelimiter(e.target.value)}
            className={inputClass}
            aria-label="CSV delimiter"
          >
            <option value=",">, (comma)</option>
            <option value=";">; (semicolon)</option>
            <option value={"\t"}>Tab</option>
            <option value="|">| (pipe)</option>
          </select>
        </div>
      </div>

      <div className="space-y-1">
        <div className="flex items-center justify-between">
          <label className="text-xs text-muted-foreground">
            CSV Content (first row = column headers)
          </label>
          <button
            type="button"
            onClick={() => void handleFileLoad()}
            className={cn(
              "flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground",
              "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring rounded px-1"
            )}
          >
            <Upload className="h-3 w-3" />
            Load File
          </button>
        </div>
        <textarea
          value={csvText}
          onChange={(e) => setCsvText(e.target.value)}
          placeholder={"username,email,token\nalice,alice@test.com,tok-abc\nbob,bob@test.com,tok-xyz"}
          rows={6}
          className={cn(
            inputClass,
            "font-mono resize-y min-h-[80px]"
          )}
          aria-label="CSV content"
        />
      </div>

      {/* Preview */}
      {previewCols.length > 0 && csvText.trim() && (
        <div className="space-y-1">
          <span className="text-[10px] text-muted-foreground uppercase tracking-wider">
            Preview ({previewRows.length} row{previewRows.length !== 1 ? "s" : ""} shown)
          </span>
          <div className="overflow-x-auto border border-border rounded">
            <table className="text-xs w-full">
              <thead>
                <tr className="bg-muted/30 border-b border-border">
                  {previewCols.map((col, i) => (
                    <th
                      key={i}
                      className="px-2 py-1 text-left font-mono font-medium text-foreground"
                    >
                      ${"{"}
                      {col}
                      {"}"}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {previewRows.map((row, ri) => (
                  <tr key={ri} className={ri % 2 === 0 ? "bg-background" : "bg-muted/10"}>
                    {row.map((val, ci) => (
                      <td key={ci} className="px-2 py-0.5 text-muted-foreground font-mono">
                        {val}
                      </td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {error && (
        <p className="text-xs text-destructive" role="alert">
          {error}
        </p>
      )}

      <div className="flex gap-2">
        <button
          onClick={() => void handleSave()}
          disabled={saving}
          aria-label="Add CSV data source"
          className={cn(
            "flex-1 flex items-center justify-center gap-1 text-xs px-2 py-1 rounded",
            "bg-primary text-primary-foreground hover:bg-primary/90",
            "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
            "disabled:opacity-50 disabled:pointer-events-none"
          )}
        >
          <Plus className="h-3 w-3" aria-hidden="true" />
          {saving ? "Adding..." : "Add CSV Source"}
        </button>
        <button
          onClick={onCancel}
          disabled={saving}
          aria-label="Cancel"
          className={cn(
            "flex-1 flex items-center justify-center gap-1 text-xs px-2 py-1 rounded",
            "border border-input bg-background hover:bg-muted",
            "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
            "disabled:opacity-50 disabled:pointer-events-none"
          )}
        >
          Cancel
        </button>
      </div>
    </div>
  );
}

// ----------------------------------------------------------------
// CsvSourceRow — single data source display
// ----------------------------------------------------------------

interface CsvSourceRowProps {
  source: CsvDataSource;
  planId: string;
  onChanged: () => void;
}

function CsvSourceRow({ source, planId, onChanged }: CsvSourceRowProps) {
  const [removing, setRemoving] = useState(false);
  const [expanded, setExpanded] = useState(false);

  async function handleRemove() {
    setRemoving(true);
    try {
      await removeCsvDataSource(planId, source.id);
      onChanged();
    } catch {
      setRemoving(false);
    }
  }

  async function handleSharingChange(mode: CsvSharingMode) {
    try {
      await updateCsvDataSource(planId, source.id, undefined, mode);
      onChanged();
    } catch {
      // ignore
    }
  }

  async function handleRecycleChange(recycle: boolean) {
    try {
      await updateCsvDataSource(planId, source.id, undefined, undefined, recycle);
      onChanged();
    } catch {
      // ignore
    }
  }

  return (
    <div
      className={cn(
        "border border-border rounded bg-muted/10",
        "hover:bg-muted/20 transition-colors"
      )}
    >
      {/* Header row */}
      <div className="flex items-center gap-2 px-2 py-1.5 group">
        <FileSpreadsheet className="h-3.5 w-3.5 text-green-500 shrink-0" />
        <button
          onClick={() => setExpanded(!expanded)}
          className="flex-1 text-left"
        >
          <span className="text-xs font-medium text-foreground">{source.name}</span>
          <span className="ml-2 text-[10px] text-muted-foreground">
            {source.columns.length} cols, {source.rows.length} rows
          </span>
        </button>

        {/* Variables hint */}
        <span className="text-[10px] text-muted-foreground font-mono truncate max-w-[200px]" title={source.columns.map(c => `\${${c}}`).join(", ")}>
          {source.columns.slice(0, 3).map((c) => `\${${c}}`).join(", ")}
          {source.columns.length > 3 && "..."}
        </span>

        {/* Remove */}
        <button
          onClick={() => void handleRemove()}
          disabled={removing}
          aria-label={`Remove ${source.name}`}
          className={cn(
            "shrink-0 p-0.5 rounded opacity-0 group-hover:opacity-100",
            "hover:bg-destructive/10 hover:text-destructive text-muted-foreground",
            "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring focus-visible:opacity-100",
            "disabled:opacity-50 disabled:pointer-events-none"
          )}
        >
          <Trash2 className="h-3 w-3" />
        </button>
      </div>

      {/* Expanded details */}
      {expanded && (
        <div className="border-t border-border px-3 py-2 space-y-2">
          {/* Settings */}
          <div className="flex items-center gap-3">
            <div className="space-y-0.5">
              <label className="text-[10px] text-muted-foreground">Sharing</label>
              <select
                value={source.sharing_mode}
                onChange={(e) => void handleSharingChange(e.target.value as CsvSharingMode)}
                className={cn(inputClass, "w-32")}
                aria-label="Sharing mode"
              >
                <option value="all_threads">All Threads</option>
                <option value="per_thread">Per Thread</option>
              </select>
            </div>
            <div className="space-y-0.5">
              <label className="text-[10px] text-muted-foreground">On EOF</label>
              <select
                value={source.recycle ? "recycle" : "stop"}
                onChange={(e) => void handleRecycleChange(e.target.value === "recycle")}
                className={cn(inputClass, "w-28")}
                aria-label="Recycle on EOF"
              >
                <option value="recycle">Recycle</option>
                <option value="stop">Stop</option>
              </select>
            </div>
          </div>

          {/* Data preview */}
          <div className="overflow-x-auto border border-border rounded max-h-40">
            <table className="text-[10px] w-full">
              <thead>
                <tr className="bg-muted/30 border-b border-border sticky top-0">
                  <th className="px-1.5 py-0.5 text-left text-muted-foreground font-normal">#</th>
                  {source.columns.map((col, i) => (
                    <th key={i} className="px-1.5 py-0.5 text-left font-mono font-medium">
                      {col}
                    </th>
                  ))}
                </tr>
              </thead>
              <tbody>
                {source.rows.slice(0, 10).map((row, ri) => (
                  <tr key={ri} className={ri % 2 === 0 ? "bg-background" : "bg-muted/10"}>
                    <td className="px-1.5 py-0.5 text-muted-foreground">{ri + 1}</td>
                    {row.map((val, ci) => (
                      <td key={ci} className="px-1.5 py-0.5 text-muted-foreground font-mono">
                        {val}
                      </td>
                    ))}
                  </tr>
                ))}
                {source.rows.length > 10 && (
                  <tr>
                    <td
                      colSpan={source.columns.length + 1}
                      className="px-1.5 py-0.5 text-muted-foreground text-center italic"
                    >
                      ...and {source.rows.length - 10} more rows
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  );
}

// ----------------------------------------------------------------
// CsvDataSourceEditor — main exported component
// ----------------------------------------------------------------

export interface CsvDataSourceEditorProps {
  planId: string;
  csvDataSources: CsvDataSource[];
  onCsvChange: () => void;
}

export function CsvDataSourceEditor({
  planId,
  csvDataSources,
  onCsvChange,
}: CsvDataSourceEditorProps) {
  const [showAddForm, setShowAddForm] = useState(false);

  function handleAdded() {
    setShowAddForm(false);
    onCsvChange();
  }

  return (
    <div className="space-y-2">
      {/* Section header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="text-xs font-medium text-muted-foreground">CSV Data Sources</span>
          {csvDataSources.length > 0 && (
            <span className="text-xs text-muted-foreground">({csvDataSources.length})</span>
          )}
        </div>
        {!showAddForm && (
          <button
            onClick={() => setShowAddForm(true)}
            aria-label="Add CSV data source"
            className={cn(
              "flex items-center gap-1 text-xs text-muted-foreground",
              "hover:text-foreground",
              "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring rounded"
            )}
          >
            <Plus className="h-3 w-3" />
            Add CSV
          </button>
        )}
      </div>

      {/* Hint text */}
      {csvDataSources.length === 0 && !showAddForm && (
        <p className="text-xs text-muted-foreground italic">
          No CSV data sources. Add one to use dynamic variables like{" "}
          <code className="font-mono">{"${column_name}"}</code> that change per iteration.
        </p>
      )}

      {/* Source list */}
      {csvDataSources.length > 0 && (
        <div className="space-y-1" role="list" aria-label="CSV Data Sources">
          {csvDataSources.map((src) => (
            <CsvSourceRow
              key={src.id}
              source={src}
              planId={planId}
              onChanged={onCsvChange}
            />
          ))}
        </div>
      )}

      {/* Add form */}
      {showAddForm && (
        <AddCsvForm
          planId={planId}
          onAdded={handleAdded}
          onCancel={() => setShowAddForm(false)}
        />
      )}
    </div>
  );
}
