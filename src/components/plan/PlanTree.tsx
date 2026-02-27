import { useState } from "react";
import { Plus, ChevronDown, ChevronRight, FileCode2, ArrowUp, ArrowDown } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/cn";
import { usePlanStore } from "@/stores/usePlanStore";
import { ThreadGroupNode } from "./TreeNode";

interface NewGroupFormProps {
  onSubmit: (name: string) => void;
  onCancel: () => void;
}

function NewGroupForm({ onSubmit, onCancel }: NewGroupFormProps) {
  const [name, setName] = useState("Thread Group");

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const trimmed = name.trim();
    if (trimmed) onSubmit(trimmed);
  }

  return (
    <form onSubmit={handleSubmit} className="px-2 pb-2">
      <input
        autoFocus
        type="text"
        value={name}
        onChange={(e) => setName(e.target.value)}
        placeholder="Group name..."
        aria-label="New thread group name"
        className={cn(
          "w-full text-sm px-2 py-1 rounded border border-input bg-background",
          "focus:outline-none focus:ring-1 focus:ring-ring"
        )}
        onKeyDown={(e) => { if (e.key === "Escape") onCancel(); }}
      />
      <div className="flex gap-1 mt-1.5">
        <Button size="sm" type="submit" disabled={!name.trim()} className="flex-1 h-7 text-xs">
          Add
        </Button>
        <Button size="sm" type="button" variant="ghost" onClick={onCancel} className="flex-1 h-7 text-xs">
          Cancel
        </Button>
      </div>
    </form>
  );
}

export function PlanTree() {
  const {
    activePlan,
    selectedNodeId,
    selectNode,
    addThreadGroup,
    reorderThreadGroups,
  } = usePlanStore();

  const [planExpanded, setPlanExpanded] = useState(true);
  const [showNewGroupForm, setShowNewGroupForm] = useState(false);

  async function handleAddGroup(name: string) {
    setShowNewGroupForm(false);
    await addThreadGroup(name);
    setPlanExpanded(true);
  }

  function moveGroupUp(index: number) {
    if (!activePlan || index === 0) return;
    const ids = activePlan.thread_groups.map((g) => g.id);
    const newIds = [...ids];
    [newIds[index - 1], newIds[index]] = [newIds[index], newIds[index - 1]];
    void reorderThreadGroups(newIds);
  }

  function moveGroupDown(index: number) {
    if (!activePlan || index === activePlan.thread_groups.length - 1) return;
    const ids = activePlan.thread_groups.map((g) => g.id);
    const newIds = [...ids];
    [newIds[index], newIds[index + 1]] = [newIds[index + 1], newIds[index]];
    void reorderThreadGroups(newIds);
  }

  if (!activePlan) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center gap-3 text-muted-foreground px-4">
        <FileCode2 className="h-10 w-10" aria-hidden="true" />
        <p className="text-sm text-center">Select or create a test plan to get started.</p>
      </div>
    );
  }

  const isPlanSelected = selectedNodeId === activePlan.id;
  const ChevronIcon = planExpanded ? ChevronDown : ChevronRight;

  return (
    <div className="flex-1 overflow-y-auto p-2" role="tree" aria-label="Test plan tree">
      {/* Plan root node */}
      <div
        role="treeitem"
        aria-selected={isPlanSelected}
        aria-expanded={planExpanded}
        tabIndex={0}
        className={cn(
          "flex items-center gap-1.5 px-2 py-1.5 rounded-md cursor-pointer select-none",
          "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
          isPlanSelected
            ? "bg-primary text-primary-foreground"
            : "hover:bg-accent hover:text-accent-foreground"
        )}
        onClick={() => selectNode(activePlan.id, "plan")}
        onKeyDown={(e) => {
          if (e.key === "Enter") selectNode(activePlan.id, "plan");
          if (e.key === " ") { e.preventDefault(); setPlanExpanded((v) => !v); }
          if (e.key === "ArrowRight") setPlanExpanded(true);
          if (e.key === "ArrowLeft") setPlanExpanded(false);
        }}
      >
        <button
          aria-label={planExpanded ? "Collapse plan" : "Expand plan"}
          onClick={(e) => { e.stopPropagation(); setPlanExpanded((v) => !v); }}
          className="shrink-0 rounded hover:bg-accent/50 p-0.5 -ml-0.5"
          tabIndex={-1}
        >
          <ChevronIcon className="h-3.5 w-3.5" aria-hidden="true" />
        </button>
        <FileCode2 className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
        <span className="text-sm font-semibold truncate flex-1">{activePlan.name}</span>
      </div>

      {/* Thread groups */}
      {planExpanded && (
        <div role="group" className="mt-0.5 space-y-0.5">
          {activePlan.thread_groups.map((group, idx) => (
            <div key={group.id} className="group/row flex items-start gap-0.5">
              <div className="flex-1 min-w-0">
                <ThreadGroupNode group={group} depth={1} />
              </div>
              {/* Reorder buttons — shown on hover */}
              <div className="flex flex-col shrink-0 opacity-0 group-hover/row:opacity-100 transition-opacity">
                <button
                  aria-label={`Move ${group.name} up`}
                  onClick={() => moveGroupUp(idx)}
                  disabled={idx === 0}
                  className={cn(
                    "p-0.5 rounded hover:bg-accent hover:text-accent-foreground",
                    "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
                    "disabled:opacity-30 disabled:pointer-events-none"
                  )}
                >
                  <ArrowUp className="h-3 w-3" />
                </button>
                <button
                  aria-label={`Move ${group.name} down`}
                  onClick={() => moveGroupDown(idx)}
                  disabled={idx === activePlan.thread_groups.length - 1}
                  className={cn(
                    "p-0.5 rounded hover:bg-accent hover:text-accent-foreground",
                    "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
                    "disabled:opacity-30 disabled:pointer-events-none"
                  )}
                >
                  <ArrowDown className="h-3 w-3" />
                </button>
              </div>
            </div>
          ))}

          {/* Add thread group */}
          {showNewGroupForm ? (
            <NewGroupForm
              onSubmit={(name) => void handleAddGroup(name)}
              onCancel={() => setShowNewGroupForm(false)}
            />
          ) : (
            <button
              onClick={() => setShowNewGroupForm(true)}
              className={cn(
                "w-full flex items-center gap-1.5 px-2 py-1 rounded-md text-xs text-muted-foreground",
                "hover:bg-accent hover:text-accent-foreground",
                "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
              )}
              style={{ paddingLeft: 28 }}
            >
              <Plus className="h-3.5 w-3.5" aria-hidden="true" />
              Add Thread Group
            </button>
          )}
        </div>
      )}
    </div>
  );
}
