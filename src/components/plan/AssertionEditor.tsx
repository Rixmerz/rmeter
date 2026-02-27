import { useState } from "react";
import { Plus, Trash2, CheckCircle2, XCircle } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/cn";
import type { Assertion, AssertionRule } from "@/types/plan";
import type { AssertionResult } from "@/types/results";
import {
  addAssertion as addAssertionCmd,
  removeAssertion as removeAssertionCmd,
} from "@/lib/commands";

// ----------------------------------------------------------------
// Constants
// ----------------------------------------------------------------

type AssertionRuleType = AssertionRule["type"];

const RULE_TYPE_LABELS: Record<AssertionRuleType, string> = {
  status_code_equals: "Status Code Equals",
  status_code_not_equals: "Status Code Not Equals",
  status_code_range: "Status Code Range",
  body_contains: "Body Contains",
  body_not_contains: "Body Not Contains",
  json_path: "JSON Path",
  response_time_below: "Response Time Below",
  header_equals: "Header Equals",
  header_contains: "Header Contains",
};

const RULE_TYPE_ORDER: AssertionRuleType[] = [
  "status_code_equals",
  "status_code_not_equals",
  "status_code_range",
  "body_contains",
  "body_not_contains",
  "json_path",
  "response_time_below",
  "header_equals",
  "header_contains",
];

// ----------------------------------------------------------------
// Default rule values per type
// ----------------------------------------------------------------

function defaultRuleForType(type: AssertionRuleType): AssertionRule {
  switch (type) {
    case "status_code_equals":
      return { type, expected: 200 };
    case "status_code_not_equals":
      return { type, not_expected: 500 };
    case "status_code_range":
      return { type, min: 200, max: 299 };
    case "body_contains":
      return { type, substring: "" };
    case "body_not_contains":
      return { type, substring: "" };
    case "json_path":
      return { type, expression: "$.status", expected: "ok" };
    case "response_time_below":
      return { type, threshold_ms: 1000 };
    case "header_equals":
      return { type, header: "Content-Type", expected: "application/json" };
    case "header_contains":
      return { type, header: "Content-Type", substring: "json" };
  }
}

// ----------------------------------------------------------------
// Shared input style
// ----------------------------------------------------------------

const inputClass = cn(
  "w-full text-xs px-2 py-1 rounded border border-input bg-background",
  "focus:outline-none focus:ring-1 focus:ring-ring"
);

// ----------------------------------------------------------------
// Rule description renderer (read-only summary)
// ----------------------------------------------------------------

function ruleDescription(rule: AssertionRule): string {
  switch (rule.type) {
    case "status_code_equals":
      return `= ${rule.expected}`;
    case "status_code_not_equals":
      return `!= ${rule.not_expected}`;
    case "status_code_range":
      return `${rule.min}–${rule.max}`;
    case "body_contains":
    case "body_not_contains":
      return `"${rule.substring}"`;
    case "json_path":
      return `${rule.expression} = ${JSON.stringify(rule.expected)}`;
    case "response_time_below":
      return `< ${rule.threshold_ms} ms`;
    case "header_equals":
      return `${rule.header}: ${rule.expected}`;
    case "header_contains":
      return `${rule.header} contains "${rule.substring}"`;
  }
}

// ----------------------------------------------------------------
// RuleFields — dynamic form fields based on rule type
// ----------------------------------------------------------------

interface RuleFieldsProps {
  rule: AssertionRule;
  onChange: (rule: AssertionRule) => void;
}

function RuleFields({ rule, onChange }: RuleFieldsProps) {
  switch (rule.type) {
    case "status_code_equals":
      return (
        <div className="space-y-1">
          <label className="text-xs text-muted-foreground">Expected Status</label>
          <input
            type="number"
            min={100}
            max={599}
            value={rule.expected}
            onChange={(e) => onChange({ ...rule, expected: Number(e.target.value) })}
            className={inputClass}
            aria-label="Expected status code"
          />
        </div>
      );

    case "status_code_not_equals":
      return (
        <div className="space-y-1">
          <label className="text-xs text-muted-foreground">Not Expected Status</label>
          <input
            type="number"
            min={100}
            max={599}
            value={rule.not_expected}
            onChange={(e) => onChange({ ...rule, not_expected: Number(e.target.value) })}
            className={inputClass}
            aria-label="Not expected status code"
          />
        </div>
      );

    case "status_code_range":
      return (
        <div className="flex gap-2">
          <div className="flex-1 space-y-1">
            <label className="text-xs text-muted-foreground">Min</label>
            <input
              type="number"
              min={100}
              max={599}
              value={rule.min}
              onChange={(e) => onChange({ ...rule, min: Number(e.target.value) })}
              className={inputClass}
              aria-label="Minimum status code"
            />
          </div>
          <div className="flex-1 space-y-1">
            <label className="text-xs text-muted-foreground">Max</label>
            <input
              type="number"
              min={100}
              max={599}
              value={rule.max}
              onChange={(e) => onChange({ ...rule, max: Number(e.target.value) })}
              className={inputClass}
              aria-label="Maximum status code"
            />
          </div>
        </div>
      );

    case "body_contains":
    case "body_not_contains":
      return (
        <div className="space-y-1">
          <label className="text-xs text-muted-foreground">Substring</label>
          <input
            type="text"
            value={rule.substring}
            onChange={(e) => onChange({ ...rule, substring: e.target.value })}
            placeholder='e.g. "success"'
            className={inputClass}
            aria-label="Substring to search for"
          />
        </div>
      );

    case "json_path":
      return (
        <div className="space-y-2">
          <div className="space-y-1">
            <label className="text-xs text-muted-foreground">Expression</label>
            <input
              type="text"
              value={rule.expression}
              onChange={(e) => onChange({ ...rule, expression: e.target.value })}
              placeholder="$.data.status"
              className={inputClass}
              aria-label="JSONPath expression"
            />
          </div>
          <div className="space-y-1">
            <label className="text-xs text-muted-foreground">Expected Value (JSON)</label>
            <input
              type="text"
              value={typeof rule.expected === "string" ? rule.expected : JSON.stringify(rule.expected)}
              onChange={(e) => {
                let parsed: unknown;
                try {
                  parsed = JSON.parse(e.target.value);
                } catch {
                  parsed = e.target.value;
                }
                onChange({ ...rule, expected: parsed });
              }}
              placeholder='"ok" or 200 or true'
              className={inputClass}
              aria-label="Expected JSON value"
            />
          </div>
        </div>
      );

    case "response_time_below":
      return (
        <div className="space-y-1">
          <label className="text-xs text-muted-foreground">Threshold (ms)</label>
          <input
            type="number"
            min={1}
            value={rule.threshold_ms}
            onChange={(e) => onChange({ ...rule, threshold_ms: Number(e.target.value) })}
            className={inputClass}
            aria-label="Response time threshold in milliseconds"
          />
        </div>
      );

    case "header_equals":
      return (
        <div className="space-y-2">
          <div className="space-y-1">
            <label className="text-xs text-muted-foreground">Header Name</label>
            <input
              type="text"
              value={rule.header}
              onChange={(e) => onChange({ ...rule, header: e.target.value })}
              placeholder="Content-Type"
              className={inputClass}
              aria-label="Header name"
            />
          </div>
          <div className="space-y-1">
            <label className="text-xs text-muted-foreground">Expected Value</label>
            <input
              type="text"
              value={rule.expected}
              onChange={(e) => onChange({ ...rule, expected: e.target.value })}
              placeholder="application/json"
              className={inputClass}
              aria-label="Expected header value"
            />
          </div>
        </div>
      );

    case "header_contains":
      return (
        <div className="space-y-2">
          <div className="space-y-1">
            <label className="text-xs text-muted-foreground">Header Name</label>
            <input
              type="text"
              value={rule.header}
              onChange={(e) => onChange({ ...rule, header: e.target.value })}
              placeholder="Content-Type"
              className={inputClass}
              aria-label="Header name"
            />
          </div>
          <div className="space-y-1">
            <label className="text-xs text-muted-foreground">Substring</label>
            <input
              type="text"
              value={rule.substring}
              onChange={(e) => onChange({ ...rule, substring: e.target.value })}
              placeholder="json"
              className={inputClass}
              aria-label="Header value substring"
            />
          </div>
        </div>
      );
  }
}

// ----------------------------------------------------------------
// AddAssertionForm — inline form for new assertion
// ----------------------------------------------------------------

interface AddAssertionFormProps {
  planId: string;
  groupId: string;
  requestId: string;
  onAdded: () => void;
  onCancel: () => void;
}

function AddAssertionForm({ planId, groupId, requestId, onAdded, onCancel }: AddAssertionFormProps) {
  const [name, setName] = useState("New Assertion");
  const [ruleType, setRuleType] = useState<AssertionRuleType>("status_code_equals");
  const [rule, setRule] = useState<AssertionRule>(defaultRuleForType("status_code_equals"));
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  function handleTypeChange(type: AssertionRuleType) {
    setRuleType(type);
    setRule(defaultRuleForType(type));
  }

  async function handleSave() {
    setSaving(true);
    setError(null);
    try {
      await addAssertionCmd(planId, groupId, requestId, name.trim() || "Assertion", rule);
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
          placeholder="Assertion name"
          className={inputClass}
          aria-label="Assertion name"
          autoFocus
        />
      </div>

      <div className="space-y-1">
        <label className="text-xs text-muted-foreground">Type</label>
        <select
          value={ruleType}
          onChange={(e) => handleTypeChange(e.target.value as AssertionRuleType)}
          className={cn(
            "w-full text-xs px-2 py-1 rounded border border-input bg-background",
            "focus:outline-none focus:ring-1 focus:ring-ring"
          )}
          aria-label="Assertion type"
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
        <Button
          size="sm"
          onClick={() => void handleSave()}
          disabled={saving}
          className="flex-1"
        >
          {saving ? "Adding..." : "Add"}
        </Button>
        <Button
          size="sm"
          variant="outline"
          onClick={onCancel}
          disabled={saving}
          className="flex-1"
        >
          Cancel
        </Button>
      </div>
    </div>
  );
}

// ----------------------------------------------------------------
// AssertionRow — single assertion item
// ----------------------------------------------------------------

interface AssertionRowProps {
  assertion: Assertion;
  result?: AssertionResult;
  planId: string;
  groupId: string;
  requestId: string;
  onRemoved: () => void;
}

function AssertionRow({ assertion, result, planId, groupId, requestId, onRemoved }: AssertionRowProps) {
  const [removing, setRemoving] = useState(false);

  async function handleRemove() {
    setRemoving(true);
    try {
      await removeAssertionCmd(planId, groupId, requestId, assertion.id);
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
        hasResult && result.passed
          ? "border-green-500/30 bg-green-500/5"
          : hasResult && !result.passed
          ? "border-destructive/30 bg-destructive/5"
          : "border-border bg-muted/10"
      )}
      role="listitem"
    >
      {/* Pass/fail indicator */}
      <span className="shrink-0 mt-0.5" aria-hidden="true">
        {hasResult ? (
          result.passed ? (
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
        <div className="font-medium text-foreground truncate" title={assertion.name}>
          {assertion.name}
        </div>
        <div className="text-muted-foreground">
          <span className="font-mono">{RULE_TYPE_LABELS[assertion.rule.type]}</span>
          {" — "}
          <span className="font-mono">{ruleDescription(assertion.rule)}</span>
        </div>
        {hasResult && !result.passed && result.message && (
          <div className="text-destructive truncate" title={result.message}>
            {result.message}
          </div>
        )}
      </div>

      {/* Remove button */}
      <button
        onClick={() => void handleRemove()}
        disabled={removing}
        aria-label={`Remove assertion ${assertion.name}`}
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
// AssertionEditor — main exported component
// ----------------------------------------------------------------

export interface AssertionEditorProps {
  planId: string;
  groupId: string;
  requestId: string;
  assertions: Assertion[];
  assertionResults?: AssertionResult[];
  onAssertionsChange: () => void;
}

export function AssertionEditor({
  planId,
  groupId,
  requestId,
  assertions,
  assertionResults = [],
  onAssertionsChange,
}: AssertionEditorProps) {
  const [showAddForm, setShowAddForm] = useState(false);

  // Build lookup map from assertion_id -> AssertionResult
  const resultMap = new Map<string, AssertionResult>();
  for (const r of assertionResults) {
    resultMap.set(r.assertion_id, r);
  }

  async function handleAdded() {
    setShowAddForm(false);
    onAssertionsChange();
  }

  function handleRemoved() {
    onAssertionsChange();
  }

  // Summary counts when results are available
  const passedCount = assertionResults.filter((r) => r.passed).length;
  const totalCount = assertionResults.length;

  return (
    <div className="space-y-2">
      {/* Section header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="text-xs font-medium text-muted-foreground">
            Assertions
          </span>
          {assertions.length > 0 && (
            <span className="text-xs text-muted-foreground">
              ({assertions.length})
            </span>
          )}
          {totalCount > 0 && (
            <span
              className={cn(
                "text-xs font-medium px-1.5 py-0.5 rounded",
                passedCount === totalCount
                  ? "bg-green-500/15 text-green-600 dark:text-green-400"
                  : "bg-destructive/15 text-destructive"
              )}
              aria-label={`${passedCount} of ${totalCount} assertions passed`}
            >
              {passedCount}/{totalCount} passed
            </span>
          )}
        </div>
        {!showAddForm && (
          <button
            onClick={() => setShowAddForm(true)}
            aria-label="Add assertion"
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
      {assertions.length === 0 && !showAddForm && (
        <p className="text-xs text-muted-foreground italic">No assertions</p>
      )}

      {/* Assertion list */}
      {assertions.length > 0 && (
        <div className="space-y-1.5" role="list" aria-label="Assertions">
          {assertions.map((a) => (
            <AssertionRow
              key={a.id}
              assertion={a}
              result={resultMap.get(a.id)}
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
        <AddAssertionForm
          planId={planId}
          groupId={groupId}
          requestId={requestId}
          onAdded={() => void handleAdded()}
          onCancel={() => setShowAddForm(false)}
        />
      )}
    </div>
  );
}
