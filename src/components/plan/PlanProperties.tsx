import { usePlanStore, type NodeType } from "@/stores/usePlanStore";
import { ThreadGroupProperties } from "./ThreadGroupProperties";
import { RequestProperties } from "./RequestProperties";
import { VariableEditor } from "./VariableEditor";
import { CsvDataSourceEditor } from "./CsvDataSourceEditor";
import { MousePointerClick } from "lucide-react";

function PlanRootProperties() {
  const { activePlan, loadPlans } = usePlanStore();
  if (!activePlan) return null;

  return (
    <div className="space-y-4">
      <h3 className="text-sm font-semibold">Test Plan</h3>

      <div className="space-y-1">
        <span className="text-xs font-medium text-muted-foreground block">Name</span>
        <p className="text-sm font-medium">{activePlan.name}</p>
      </div>

      {activePlan.description && (
        <div className="space-y-1">
          <span className="text-xs font-medium text-muted-foreground block">Description</span>
          <p className="text-sm text-muted-foreground">{activePlan.description}</p>
        </div>
      )}

      <div className="grid grid-cols-2 gap-3">
        <div className="space-y-0.5">
          <span className="text-xs text-muted-foreground">Thread Groups</span>
          <p className="text-lg font-bold tabular-nums">{activePlan.thread_groups.length}</p>
        </div>
        <div className="space-y-0.5">
          <span className="text-xs text-muted-foreground">Total Requests</span>
          <p className="text-lg font-bold tabular-nums">
            {activePlan.thread_groups.reduce((sum, g) => sum + g.requests.length, 0)}
          </p>
        </div>
      </div>

      <VariableEditor
        planId={activePlan.id}
        variables={activePlan.variables}
        onVariablesChange={() => void loadPlans()}
      />

      <CsvDataSourceEditor
        planId={activePlan.id}
        csvDataSources={activePlan.csv_data_sources ?? []}
        onCsvChange={() => void loadPlans()}
      />

      <div className="space-y-1">
        <span className="text-xs font-medium text-muted-foreground block">Format Version</span>
        <p className="text-sm tabular-nums">{activePlan.format_version}</p>
      </div>
    </div>
  );
}

function EmptyState() {
  return (
    <div className="flex flex-col items-center justify-center h-full gap-3 text-muted-foreground px-4 text-center">
      <MousePointerClick className="h-8 w-8" aria-hidden="true" />
      <p className="text-sm">Select a node in the tree to view its properties.</p>
    </div>
  );
}

export function PlanProperties() {
  const { activePlan, selectedNodeId, selectedNodeType } = usePlanStore();

  if (!activePlan || !selectedNodeId || !selectedNodeType) {
    return (
      <div className="flex-1 overflow-y-auto p-4">
        <EmptyState />
      </div>
    );
  }

  function renderContent(nodeId: string, nodeType: NodeType) {
    if (!activePlan) return null;

    if (nodeType === "plan") {
      return <PlanRootProperties />;
    }

    if (nodeType === "thread_group") {
      const group = activePlan.thread_groups.find((g) => g.id === nodeId);
      if (!group) return <EmptyState />;
      return <ThreadGroupProperties group={group} />;
    }

    if (nodeType === "request") {
      for (const group of activePlan.thread_groups) {
        const request = group.requests.find((r) => r.id === nodeId);
        if (request) {
          return <RequestProperties request={request} groupId={group.id} />;
        }
      }
      return <EmptyState />;
    }

    return <EmptyState />;
  }

  return (
    <div className="flex-1 overflow-y-auto p-4">
      {renderContent(selectedNodeId, selectedNodeType)}
    </div>
  );
}
