import { useState } from "react";
import { Plus, Trash2, Check, X } from "lucide-react";
import { cn } from "@/lib/cn";
import type { Variable } from "@/types/plan";
import {
  addVariable as addVariableCmd,
  removeVariable as removeVariableCmd,
  updateVariable as updateVariableCmd,
} from "@/lib/commands";

// ----------------------------------------------------------------
// Shared input style
// ----------------------------------------------------------------

const inputClass = cn(
  "w-full text-xs px-2 py-1 rounded border border-input bg-background",
  "focus:outline-none focus:ring-1 focus:ring-ring"
);

// ----------------------------------------------------------------
// Scope labels and options
// ----------------------------------------------------------------

const SCOPE_OPTIONS: Array<{ value: Variable["scope"]; label: string }> = [
  { value: "plan", label: "Plan" },
  { value: "thread_group", label: "Thread Group" },
  { value: "global", label: "Global" },
];

const SCOPE_BADGE_CLASS: Record<Variable["scope"], string> = {
  plan: "bg-blue-500/15 text-blue-600 dark:text-blue-400",
  thread_group: "bg-purple-500/15 text-purple-600 dark:text-purple-400",
  global: "bg-orange-500/15 text-orange-600 dark:text-orange-400",
};

// ----------------------------------------------------------------
// AddVariableForm — inline form for new variable
// ----------------------------------------------------------------

interface AddVariableFormProps {
  planId: string;
  onAdded: () => void;
  onCancel: () => void;
}

function AddVariableForm({ planId, onAdded, onCancel }: AddVariableFormProps) {
  const [name, setName] = useState("");
  const [value, setValue] = useState("");
  const [scope, setScope] = useState<Variable["scope"]>("plan");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function handleSave() {
    const trimmedName = name.trim();
    if (!trimmedName) {
      setError("Variable name is required");
      return;
    }
    setSaving(true);
    setError(null);
    try {
      await addVariableCmd(planId, trimmedName, value, scope);
      onAdded();
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
      setSaving(false);
    }
  }

  return (
    <div className="border border-border rounded p-3 space-y-3 bg-muted/20">
      <div className="grid grid-cols-2 gap-2">
        <div className="space-y-1">
          <label className="text-xs text-muted-foreground">Name</label>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="variableName"
            className={inputClass}
            aria-label="Variable name"
            autoFocus
          />
        </div>
        <div className="space-y-1">
          <label className="text-xs text-muted-foreground">Value</label>
          <input
            type="text"
            value={value}
            onChange={(e) => setValue(e.target.value)}
            placeholder="value"
            className={inputClass}
            aria-label="Variable value"
          />
        </div>
      </div>

      <div className="space-y-1">
        <label className="text-xs text-muted-foreground">Scope</label>
        <select
          value={scope}
          onChange={(e) => setScope(e.target.value as Variable["scope"])}
          className={cn(
            "w-full text-xs px-2 py-1 rounded border border-input bg-background",
            "focus:outline-none focus:ring-1 focus:ring-ring"
          )}
          aria-label="Variable scope"
        >
          {SCOPE_OPTIONS.map((o) => (
            <option key={o.value} value={o.value}>
              {o.label}
            </option>
          ))}
        </select>
      </div>

      {error && (
        <p className="text-xs text-destructive" role="alert">
          {error}
        </p>
      )}

      <div className="flex gap-2">
        <button
          onClick={() => void handleSave()}
          disabled={saving}
          aria-label="Save variable"
          className={cn(
            "flex-1 flex items-center justify-center gap-1 text-xs px-2 py-1 rounded",
            "bg-primary text-primary-foreground hover:bg-primary/90",
            "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
            "disabled:opacity-50 disabled:pointer-events-none"
          )}
        >
          <Check className="h-3 w-3" aria-hidden="true" />
          {saving ? "Adding..." : "Add"}
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
          <X className="h-3 w-3" aria-hidden="true" />
          Cancel
        </button>
      </div>
    </div>
  );
}

// ----------------------------------------------------------------
// VariableRow — single variable item with inline editing
// ----------------------------------------------------------------

interface VariableRowProps {
  variable: Variable;
  planId: string;
  onChanged: () => void;
}

function VariableRow({ variable, planId, onChanged }: VariableRowProps) {
  const [editing, setEditing] = useState(false);
  const [name, setName] = useState(variable.name);
  const [value, setValue] = useState(variable.value);
  const [scope, setScope] = useState<Variable["scope"]>(variable.scope);
  const [saving, setSaving] = useState(false);
  const [removing, setRemoving] = useState(false);

  async function handleSave() {
    setSaving(true);
    try {
      await updateVariableCmd(planId, variable.id, name.trim() || variable.name, value, scope);
      setEditing(false);
      onChanged();
    } catch {
      // Keep form open on error
    } finally {
      setSaving(false);
    }
  }

  function handleDiscard() {
    setName(variable.name);
    setValue(variable.value);
    setScope(variable.scope);
    setEditing(false);
  }

  async function handleRemove() {
    setRemoving(true);
    try {
      await removeVariableCmd(planId, variable.id);
      onChanged();
    } catch {
      setRemoving(false);
    }
  }

  if (editing) {
    return (
      <div className="border border-primary/30 rounded p-2 space-y-2 bg-muted/20" role="listitem">
        <div className="grid grid-cols-2 gap-1.5">
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="variableName"
            className={inputClass}
            aria-label="Variable name"
            autoFocus
          />
          <input
            type="text"
            value={value}
            onChange={(e) => setValue(e.target.value)}
            placeholder="value"
            className={inputClass}
            aria-label="Variable value"
          />
        </div>
        <div className="flex items-center gap-2">
          <select
            value={scope}
            onChange={(e) => setScope(e.target.value as Variable["scope"])}
            className={cn(
              "flex-1 text-xs px-2 py-1 rounded border border-input bg-background",
              "focus:outline-none focus:ring-1 focus:ring-ring"
            )}
            aria-label="Variable scope"
          >
            {SCOPE_OPTIONS.map((o) => (
              <option key={o.value} value={o.value}>
                {o.label}
              </option>
            ))}
          </select>
          <button
            onClick={() => void handleSave()}
            disabled={saving}
            aria-label="Save changes"
            className={cn(
              "p-1 rounded hover:bg-green-500/10 hover:text-green-600",
              "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
              "disabled:opacity-50 disabled:pointer-events-none"
            )}
          >
            <Check className="h-3.5 w-3.5" />
          </button>
          <button
            onClick={handleDiscard}
            disabled={saving}
            aria-label="Discard changes"
            className={cn(
              "p-1 rounded hover:bg-muted",
              "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
              "disabled:opacity-50 disabled:pointer-events-none"
            )}
          >
            <X className="h-3.5 w-3.5" />
          </button>
        </div>
      </div>
    );
  }

  return (
    <div
      className={cn(
        "flex items-center gap-2 px-2 py-1.5 rounded border border-border bg-muted/10",
        "hover:bg-muted/20 transition-colors group cursor-default"
      )}
      role="listitem"
    >
      {/* Name */}
      <span
        className="font-mono text-xs font-medium text-foreground truncate min-w-0 flex-1"
        title={`\${${variable.name}}`}
      >
        {variable.name}
      </span>

      {/* Value */}
      <span
        className="text-xs text-muted-foreground truncate min-w-0 flex-1"
        title={variable.value}
      >
        {variable.value || <em className="opacity-60">empty</em>}
      </span>

      {/* Scope badge */}
      <span
        className={cn(
          "shrink-0 text-[10px] font-medium px-1.5 py-0.5 rounded",
          SCOPE_BADGE_CLASS[variable.scope]
        )}
        aria-label={`Scope: ${variable.scope}`}
      >
        {variable.scope}
      </span>

      {/* Edit button */}
      <button
        onClick={() => setEditing(true)}
        aria-label={`Edit variable ${variable.name}`}
        className={cn(
          "shrink-0 p-0.5 rounded opacity-0 group-hover:opacity-100",
          "hover:bg-muted hover:text-foreground text-muted-foreground",
          "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring focus-visible:opacity-100"
        )}
      >
        <Check className="h-3 w-3" aria-hidden="true" />
      </button>

      {/* Remove button */}
      <button
        onClick={() => void handleRemove()}
        disabled={removing}
        aria-label={`Remove variable ${variable.name}`}
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
  );
}

// ----------------------------------------------------------------
// VariableEditor — main exported component
// ----------------------------------------------------------------

export interface VariableEditorProps {
  planId: string;
  variables: Variable[];
  onVariablesChange: () => void;
}

export function VariableEditor({
  planId,
  variables,
  onVariablesChange,
}: VariableEditorProps) {
  const [showAddForm, setShowAddForm] = useState(false);

  function handleAdded() {
    setShowAddForm(false);
    onVariablesChange();
  }

  return (
    <div className="space-y-2">
      {/* Section header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="text-xs font-medium text-muted-foreground">Variables</span>
          {variables.length > 0 && (
            <span className="text-xs text-muted-foreground">({variables.length})</span>
          )}
        </div>
        {!showAddForm && (
          <button
            onClick={() => setShowAddForm(true)}
            aria-label="Add variable"
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

      {/* Hint text */}
      {variables.length === 0 && !showAddForm && (
        <p className="text-xs text-muted-foreground italic">
          No variables. Use <code className="font-mono">{"${varName}"}</code> in URLs, headers, and body to substitute values.
        </p>
      )}

      {/* Variable list */}
      {variables.length > 0 && (
        <div className="space-y-1" role="list" aria-label="Variables">
          {variables.map((v) => (
            <VariableRow
              key={v.id}
              variable={v}
              planId={planId}
              onChanged={onVariablesChange}
            />
          ))}
        </div>
      )}

      {/* Add form */}
      {showAddForm && (
        <AddVariableForm
          planId={planId}
          onAdded={handleAdded}
          onCancel={() => setShowAddForm(false)}
        />
      )}
    </div>
  );
}
