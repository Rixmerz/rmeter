import { useState } from "react";
import { Plus, FileCode2, Trash2, Loader2, AlertCircle, Play } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/cn";
import { usePlanStore } from "@/stores/usePlanStore";
import { useEngineStore } from "@/stores/useEngineStore";
import type { PlanSummary } from "@/types/plan";
import { TemplateDialog } from "./TemplateDialog";

interface NewPlanFormProps {
  onSubmit: (name: string) => void;
  onCancel: () => void;
}

function NewPlanForm({ onSubmit, onCancel }: NewPlanFormProps) {
  const [name, setName] = useState("");

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const trimmed = name.trim();
    if (trimmed) {
      onSubmit(trimmed);
    }
  }

  return (
    <form onSubmit={handleSubmit} className="px-2 pb-2">
      <input
        autoFocus
        type="text"
        value={name}
        onChange={(e) => setName(e.target.value)}
        placeholder="Plan name..."
        aria-label="New plan name"
        className={cn(
          "w-full text-sm px-2 py-1.5 rounded border border-input bg-background",
          "focus:outline-none focus:ring-1 focus:ring-ring"
        )}
        onKeyDown={(e) => {
          if (e.key === "Escape") onCancel();
        }}
      />
      <div className="flex gap-1 mt-1.5">
        <Button size="sm" type="submit" disabled={!name.trim()} className="flex-1 h-7 text-xs">
          Create
        </Button>
        <Button size="sm" type="button" variant="ghost" onClick={onCancel} className="flex-1 h-7 text-xs">
          Cancel
        </Button>
      </div>
    </form>
  );
}

interface PlanItemProps {
  plan: PlanSummary;
  isActive: boolean;
  onSelect: () => void;
  onDelete: () => void;
  onRun: () => void;
  isEngineRunning: boolean;
}

function PlanItem({ plan, isActive, onSelect, onDelete, onRun, isEngineRunning }: PlanItemProps) {
  const [confirmDelete, setConfirmDelete] = useState(false);

  function handleDeleteClick(e: React.MouseEvent) {
    e.stopPropagation();
    setConfirmDelete(true);
  }

  function handleConfirmDelete(e: React.MouseEvent) {
    e.stopPropagation();
    onDelete();
    setConfirmDelete(false);
  }

  function handleCancelDelete(e: React.MouseEvent) {
    e.stopPropagation();
    setConfirmDelete(false);
  }

  return (
    <div
      role="button"
      tabIndex={0}
      onClick={onSelect}
      onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") onSelect(); }}
      aria-pressed={isActive}
      className={cn(
        "group relative flex flex-col gap-0.5 px-3 py-2 rounded-md cursor-pointer",
        "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
        "transition-colors",
        isActive
          ? "bg-primary text-primary-foreground"
          : "hover:bg-accent hover:text-accent-foreground text-foreground"
      )}
    >
      <div className="flex items-center justify-between gap-2">
        <div className="flex items-center gap-2 min-w-0">
          <FileCode2
            className="h-3.5 w-3.5 shrink-0"
            aria-hidden="true"
          />
          <span className="text-sm font-medium truncate">{plan.name}</span>
        </div>
        {!confirmDelete && (
          <div className="flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity shrink-0">
            <button
              aria-label={`Run plan ${plan.name}`}
              onClick={(e) => { e.stopPropagation(); onRun(); }}
              disabled={isEngineRunning}
              className={cn(
                "rounded p-0.5",
                "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
                "disabled:opacity-40 disabled:cursor-not-allowed",
                isActive
                  ? "hover:bg-primary-foreground/20 text-primary-foreground"
                  : "hover:bg-green-500/10 text-green-600 dark:text-green-400"
              )}
            >
              <Play className="h-3.5 w-3.5" />
            </button>
            <button
              aria-label={`Delete plan ${plan.name}`}
              onClick={handleDeleteClick}
              className={cn(
                "rounded p-0.5",
                "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
                isActive
                  ? "hover:bg-primary-foreground/20 text-primary-foreground"
                  : "hover:bg-destructive/10 hover:text-destructive"
              )}
            >
              <Trash2 className="h-3.5 w-3.5" />
            </button>
          </div>
        )}
      </div>
      <span
        className={cn(
          "text-xs",
          isActive ? "text-primary-foreground/70" : "text-muted-foreground"
        )}
      >
        {plan.thread_group_count} group{plan.thread_group_count !== 1 ? "s" : ""},&nbsp;
        {plan.request_count} request{plan.request_count !== 1 ? "s" : ""}
      </span>

      {confirmDelete && (
        <div
          className="mt-1.5 flex items-center gap-1"
          onClick={(e) => e.stopPropagation()}
        >
          <span className="text-xs flex-1">Delete this plan?</span>
          <button
            onClick={handleConfirmDelete}
            aria-label="Confirm delete"
            className="text-xs px-1.5 py-0.5 rounded bg-destructive text-destructive-foreground hover:bg-destructive/90"
          >
            Delete
          </button>
          <button
            onClick={handleCancelDelete}
            aria-label="Cancel delete"
            className={cn(
              "text-xs px-1.5 py-0.5 rounded border",
              isActive
                ? "border-primary-foreground/40 hover:bg-primary-foreground/10"
                : "border-border hover:bg-accent"
            )}
          >
            Cancel
          </button>
        </div>
      )}
    </div>
  );
}

export function PlanList() {
  const {
    plans,
    activePlan,
    loading,
    error,
    selectPlan,
    createPlan,
    createFromTemplate,
    deletePlan,
  } = usePlanStore();

  const engineStatus = useEngineStore((s) => s.status);
  const startTest = useEngineStore((s) => s.startTest);
  const isEngineRunning = engineStatus === "running" || engineStatus === "stopping";

  const [showNewForm, setShowNewForm] = useState(false);
  const [showTemplateDialog, setShowTemplateDialog] = useState(false);

  async function handleCreate(name: string) {
    setShowNewForm(false);
    await createPlan(name);
  }

  async function handleSelectTemplate(template: string) {
    setShowTemplateDialog(false);
    await createFromTemplate(template);
  }

  async function handleRunPlan(planId: string) {
    // Select the plan first if it's not already active
    if (activePlan?.id !== planId) {
      await selectPlan(planId);
    }
    await startTest(planId);
  }

  return (
    <aside className="flex flex-col border-r border-border bg-card" style={{ width: 220, minWidth: 180 }}>
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2 border-b border-border">
        <span className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
          Plans
        </span>
        <div className="flex gap-1">
          <Button
            size="icon"
            variant="ghost"
            className="h-6 w-6"
            aria-label="New plan from template"
            onClick={() => setShowTemplateDialog(true)}
            title="From template"
          >
            <FileCode2 className="h-3.5 w-3.5" />
          </Button>
          <Button
            size="icon"
            variant="ghost"
            className="h-6 w-6"
            aria-label="Create new plan"
            onClick={() => { setShowNewForm(true); }}
            title="New plan"
          >
            <Plus className="h-3.5 w-3.5" />
          </Button>
        </div>
      </div>

      {/* List */}
      <div className="flex-1 overflow-y-auto p-2 space-y-1">
        {loading && plans.length === 0 && (
          <div className="flex items-center justify-center py-8 text-muted-foreground gap-2">
            <Loader2 className="h-4 w-4 animate-spin" aria-hidden="true" />
            <span className="text-sm">Loading...</span>
          </div>
        )}

        {!loading && plans.length === 0 && !showNewForm && (
          <div className="flex flex-col items-center justify-center py-8 text-center gap-2 text-muted-foreground px-2">
            <FileCode2 className="h-8 w-8" aria-hidden="true" />
            <p className="text-xs">No test plans yet.</p>
            <p className="text-xs">Create one to get started.</p>
          </div>
        )}

        {plans.map((plan) => (
          <PlanItem
            key={plan.id}
            plan={plan}
            isActive={activePlan?.id === plan.id}
            onSelect={() => void selectPlan(plan.id)}
            onDelete={() => void deletePlan(plan.id)}
            onRun={() => void handleRunPlan(plan.id)}
            isEngineRunning={isEngineRunning}
          />
        ))}

        {showNewForm && (
          <NewPlanForm
            onSubmit={(name) => void handleCreate(name)}
            onCancel={() => setShowNewForm(false)}
          />
        )}
      </div>

      {/* Error */}
      {error && (
        <div className="px-3 py-2 border-t border-border flex items-center gap-2 text-destructive">
          <AlertCircle className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
          <span className="text-xs truncate">{error}</span>
        </div>
      )}

      {/* Template Dialog */}
      <TemplateDialog
        open={showTemplateDialog}
        onClose={() => setShowTemplateDialog(false)}
        onSelect={(template) => void handleSelectTemplate(template)}
      />
    </aside>
  );
}
