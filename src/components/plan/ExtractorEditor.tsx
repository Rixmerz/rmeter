import { useState } from "react";
import { Plus, Trash2, CheckCircle2, XCircle } from "lucide-react";
import { cn } from "@/lib/cn";
import type { Extractor, ExtractorRule } from "@/types/plan";
import type { ExtractionResult } from "@/types/results";
import {
  addExtractor as addExtractorCmd,
  removeExtractor as removeExtractorCmd,
} from "@/lib/commands";

// ----------------------------------------------------------------
// Shared input style
// ----------------------------------------------------------------

const inputClass = cn(
  "w-full text-xs px-2 py-1 rounded border border-input bg-background",
  "focus:outline-none focus:ring-1 focus:ring-ring"
);

// ----------------------------------------------------------------
// Extractor rule type labels and options
// ----------------------------------------------------------------

type ExtractorRuleType = ExtractorRule["type"];

const RULE_TYPE_LABELS: Record<ExtractorRuleType, string> = {
  json_path: "JSON Path",
  regex: "Regex",
  header: "Header",
};

const RULE_TYPE_ORDER: ExtractorRuleType[] = ["json_path", "regex", "header"];

// ----------------------------------------------------------------
// Default rule for a given type
// ----------------------------------------------------------------

function defaultRuleForType(type: ExtractorRuleType): ExtractorRule {
  switch (type) {
    case "json_path":
      return { type: "json_path", expression: "" };
    case "regex":
      return { type: "regex", pattern: "", group: 0 };
    case "header":
      return { type: "header", name: "" };
  }
}

// ----------------------------------------------------------------
// Rule description for read-only display
// ----------------------------------------------------------------

function ruleDescription(rule: ExtractorRule): string {
  switch (rule.type) {
    case "json_path":
      return rule.expression || "(expression)";
    case "regex":
      return `/${rule.pattern || "(pattern)"}/ [${rule.group}]`;
    case "header":
      return rule.name || "(header name)";
  }
}

// ----------------------------------------------------------------
// RuleFields — dynamic form fields based on extractor rule type
// ----------------------------------------------------------------

interface RuleFieldsProps {
  rule: ExtractorRule;
  onChange: (rule: ExtractorRule) => void;
}

function RuleFields({ rule, onChange }: RuleFieldsProps) {
  switch (rule.type) {
    case "json_path":
      return (
        <div className="space-y-1">
          <label className="text-xs text-muted-foreground">Expression (dot-notation)</label>
          <input
            type="text"
            value={rule.expression}
            onChange={(e) => onChange({ ...rule, expression: e.target.value })}
            placeholder="data.user.id"
            className={inputClass}
            aria-label="JSON path expression"
          />
        </div>
      );

    case "regex":
      return (
        <div className="space-y-2">
          <div className="space-y-1">
            <label className="text-xs text-muted-foreground">Pattern</label>
            <input
              type="text"
              value={rule.pattern}
              onChange={(e) => onChange({ ...rule, pattern: e.target.value })}
              placeholder='e.g. "id":(\d+)'
              className={inputClass}
              aria-label="Regex pattern"
            />
          </div>
          <div className="space-y-1">
            <label className="text-xs text-muted-foreground">Capture Group</label>
            <input
              type="number"
              min={0}
              value={rule.group}
              onChange={(e) => onChange({ ...rule, group: Number(e.target.value) })}
              className={inputClass}
              aria-label="Regex capture group index"
            />
          </div>
        </div>
      );

    case "header":
      return (
        <div className="space-y-1">
          <label className="text-xs text-muted-foreground">Header Name</label>
          <input
            type="text"
            value={rule.name}
            onChange={(e) => onChange({ ...rule, name: e.target.value })}
            placeholder="X-Request-Id"
            className={inputClass}
            aria-label="Header name to extract"
          />
        </div>
      );
  }
}

// ----------------------------------------------------------------
// AddExtractorForm — inline form for new extractor
// ----------------------------------------------------------------

interface AddExtractorFormProps {
  planId: string;
  groupId: string;
  requestId: string;
  onAdded: () => void;
  onCancel: () => void;
}

function AddExtractorForm({
  planId,
  groupId,
  requestId,
  onAdded,
  onCancel,
}: AddExtractorFormProps) {
  const [name, setName] = useState("New Extractor");
  const [variable, setVariable] = useState("");
  const [ruleType, setRuleType] = useState<ExtractorRuleType>("json_path");
  const [rule, setRule] = useState<ExtractorRule>(defaultRuleForType("json_path"));
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function handleTypeChange(type: ExtractorRuleType) {
    setRuleType(type);
    setRule(defaultRuleForType(type));
  }

  async function handleSave() {
    const trimmedVariable = variable.trim();
    if (!trimmedVariable) {
      setError("Variable name is required");
      return;
    }
    setSaving(true);
    setError(null);
    try {
      await addExtractorCmd(
        planId,
        groupId,
        requestId,
        name.trim() || "Extractor",
        trimmedVariable,
        rule
      );
      onAdded();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="border border-border rounded p-3 space-y-3 bg-muted/20">
      <div className="space-y-1">
        <label className="text-xs text-muted-foreground">Name</label>
        <input
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder="Extractor name"
          className={inputClass}
          aria-label="Extractor name"
          autoFocus
        />
      </div>

      <div className="space-y-1">
        <label className="text-xs text-muted-foreground">
          Store into Variable (<code className="font-mono">{"${name}"}</code> syntax)
        </label>
        <input
          type="text"
          value={variable}
          onChange={(e) => setVariable(e.target.value)}
          placeholder="userId"
          className={inputClass}
          aria-label="Variable name to store the extracted value"
        />
      </div>

      <div className="space-y-1">
        <label className="text-xs text-muted-foreground">Type</label>
        <select
          value={ruleType}
          onChange={(e) => handleTypeChange(e.target.value as ExtractorRuleType)}
          className={cn(
            "w-full text-xs px-2 py-1 rounded border border-input bg-background",
            "focus:outline-none focus:ring-1 focus:ring-ring"
          )}
          aria-label="Extractor type"
        >
          {RULE_TYPE_ORDER.map((t) => (
            <option key={t} value={t}>
              {RULE_TYPE_LABELS[t]}
            </option>
          ))}
        </select>
      </div>

      <RuleFields rule={rule} onChange={setRule} />

      {error && (
        <p className="text-xs text-destructive" role="alert">
          {error}
        </p>
      )}

      <div className="flex gap-2">
        <button
          onClick={() => void handleSave()}
          disabled={saving}
          aria-label="Add extractor"
          className={cn(
            "flex-1 text-xs px-2 py-1 rounded",
            "bg-primary text-primary-foreground hover:bg-primary/90",
            "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
            "disabled:opacity-50 disabled:pointer-events-none"
          )}
        >
          {saving ? "Adding..." : "Add"}
        </button>
        <button
          onClick={onCancel}
          disabled={saving}
          aria-label="Cancel"
          className={cn(
            "flex-1 text-xs px-2 py-1 rounded",
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
// ExtractorRow — single extractor item
// ----------------------------------------------------------------

interface ExtractorRowProps {
  extractor: Extractor;
  result?: ExtractionResult;
  planId: string;
  groupId: string;
  requestId: string;
  onRemoved: () => void;
}

function ExtractorRow({
  extractor,
  result,
  planId,
  groupId,
  requestId,
  onRemoved,
}: ExtractorRowProps) {
  const [removing, setRemoving] = useState(false);

  async function handleRemove() {
    setRemoving(true);
    try {
      await removeExtractorCmd(planId, groupId, requestId, extractor.id);
      onRemoved();
    } catch {
      setRemoving(false);
    }
  }

  const hasResult = result !== undefined;

  return (
    <div
      className={cn(
        "flex items-start gap-2 px-2 py-1.5 rounded border text-xs",
        hasResult && result.success
          ? "border-green-500/30 bg-green-500/5"
          : hasResult && !result.success
          ? "border-destructive/30 bg-destructive/5"
          : "border-border bg-muted/10"
      )}
      role="listitem"
    >
      {/* Success/fail indicator */}
      <span className="shrink-0 mt-0.5" aria-hidden="true">
        {hasResult ? (
          result.success ? (
            <CheckCircle2 className="h-3.5 w-3.5 text-green-500" />
          ) : (
            <XCircle className="h-3.5 w-3.5 text-destructive" />
          )
        ) : (
          <span className="h-3.5 w-3.5 block rounded-full border border-muted-foreground/40" />
        )}
      </span>

      {/* Content */}
      <div className="flex-1 min-w-0 space-y-0.5">
        <div className="font-medium text-foreground truncate" title={extractor.name}>
          {extractor.name}
        </div>
        <div className="text-muted-foreground">
          <span className="font-mono text-primary/80">{`\${${extractor.variable}}`}</span>
          {" <- "}
          <span className="font-mono">{RULE_TYPE_LABELS[extractor.expression.type]}</span>
          {": "}
          <span className="font-mono">{ruleDescription(extractor.expression)}</span>
        </div>
        {hasResult && result.success && result.extracted_value !== null && (
          <div className="text-green-600 dark:text-green-400 font-mono truncate" title={result.extracted_value}>
            = {result.extracted_value}
          </div>
        )}
        {hasResult && !result.success && result.message && (
          <div className="text-destructive truncate" title={result.message}>
            {result.message}
          </div>
        )}
      </div>

      {/* Remove button */}
      <button
        onClick={() => void handleRemove()}
        disabled={removing}
        aria-label={`Remove extractor ${extractor.name}`}
        className={cn(
          "shrink-0 p-0.5 rounded hover:bg-destructive/10 hover:text-destructive",
          "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
          "disabled:opacity-50 disabled:pointer-events-none"
        )}
      >
        <Trash2 className="h-3 w-3" />
      </button>
    </div>
  );
}

// ----------------------------------------------------------------
// ExtractorEditor — main exported component
// ----------------------------------------------------------------

export interface ExtractorEditorProps {
  planId: string;
  groupId: string;
  requestId: string;
  extractors: Extractor[];
  extractionResults: ExtractionResult[];
  onExtractorsChange: () => void;
}

export function ExtractorEditor({
  planId,
  groupId,
  requestId,
  extractors,
  extractionResults,
  onExtractorsChange,
}: ExtractorEditorProps) {
  const [showAddForm, setShowAddForm] = useState(false);

  // Build lookup map from extractor_id -> ExtractionResult
  const resultMap = new Map<string, ExtractionResult>();
  for (const r of extractionResults) {
    resultMap.set(r.extractor_id, r);
  }

  function handleAdded() {
    setShowAddForm(false);
    onExtractorsChange();
  }

  function handleRemoved() {
    onExtractorsChange();
  }

  // Summary counts when results are available
  const succeededCount = extractionResults.filter((r) => r.success).length;
  const totalCount = extractionResults.length;

  return (
    <div className="space-y-2">
      {/* Section header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="text-xs font-medium text-muted-foreground">Extractors</span>
          {extractors.length > 0 && (
            <span className="text-xs text-muted-foreground">({extractors.length})</span>
          )}
          {totalCount > 0 && (
            <span
              className={cn(
                "text-xs font-medium px-1.5 py-0.5 rounded",
                succeededCount === totalCount
                  ? "bg-green-500/15 text-green-600 dark:text-green-400"
                  : "bg-destructive/15 text-destructive"
              )}
              aria-label={`${succeededCount} of ${totalCount} extractions succeeded`}
            >
              {succeededCount}/{totalCount} extracted
            </span>
          )}
        </div>
        {!showAddForm && (
          <button
            onClick={() => setShowAddForm(true)}
            aria-label="Add extractor"
            className={cn(
              "flex items-center gap-1 text-xs text-muted-foreground",
              "hover:text-foreground",
              "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring rounded"
            )}
          >
            <Plus className="h-3 w-3" />
            Add
          </button>
        )}
      </div>

      {/* Empty state */}
      {extractors.length === 0 && !showAddForm && (
        <p className="text-xs text-muted-foreground italic">
          No extractors. Add one to capture values from responses into variables.
        </p>
      )}

      {/* Extractor list */}
      {extractors.length > 0 && (
        <div className="space-y-1.5" role="list" aria-label="Extractors">
          {extractors.map((ex) => (
            <ExtractorRow
              key={ex.id}
              extractor={ex}
              result={resultMap.get(ex.id)}
              planId={planId}
              groupId={groupId}
              requestId={requestId}
              onRemoved={handleRemoved}
            />
          ))}
        </div>
      )}

      {/* Add form */}
      {showAddForm && (
        <AddExtractorForm
          planId={planId}
          groupId={groupId}
          requestId={requestId}
          onAdded={handleAdded}
          onCancel={() => setShowAddForm(false)}
        />
      )}
    </div>
  );
}
