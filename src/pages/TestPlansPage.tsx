import { useEffect } from "react";
import { Save, Upload, SaveAll, Loader2 } from "lucide-react";
import { save, open } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { usePlanStore } from "@/stores/usePlanStore";
import { PlanList } from "@/components/plan/PlanList";
import { PlanTree } from "@/components/plan/PlanTree";
import { PlanProperties } from "@/components/plan/PlanProperties";

const RMETER_FILTER = [{ name: "rmeter plan", extensions: ["rmeter"] as string[] }];

export function TestPlansPage() {
  const { activePlan, loading, savePlan, loadPlanFromFile, loadPlans } = usePlanStore();

  // Load plans on mount
  useEffect(() => {
    void loadPlans();
  }, [loadPlans]);

  async function handleSave() {
    if (!activePlan) return;
    const path = await save({
      filters: [...RMETER_FILTER],
      defaultPath: `${activePlan.name}.rmeter`,
    });
    if (path) {
      await savePlan(path);
    }
  }

  async function handleSaveAs() {
    if (!activePlan) return;
    const path = await save({
      filters: [...RMETER_FILTER],
    });
    if (path) {
      await savePlan(path);
    }
  }

  async function handleLoad() {
    const path = await open({
      filters: [...RMETER_FILTER],
      multiple: false,
    });
    if (typeof path === "string") {
      await loadPlanFromFile(path);
    }
  }

  return (
    <div className="flex flex-col h-full overflow-hidden">
      {/* Page header */}
      <header className="flex items-center justify-between px-4 py-2.5 border-b border-border bg-card shrink-0">
        <h1 className="text-sm font-semibold">Test Plans</h1>
        <div className="flex items-center gap-2">
          {loading && (
            <Loader2
              className="h-4 w-4 animate-spin text-muted-foreground"
              aria-label="Loading"
            />
          )}
          <Button
            size="sm"
            variant="outline"
            onClick={() => void handleLoad()}
            className="h-7 text-xs gap-1.5"
            title="Load plan from file"
          >
            <Upload className="h-3.5 w-3.5" aria-hidden="true" />
            Load
          </Button>
          <Button
            size="sm"
            variant="outline"
            onClick={() => void handleSaveAs()}
            disabled={!activePlan}
            className="h-7 text-xs gap-1.5"
            title="Save plan as..."
          >
            <SaveAll className="h-3.5 w-3.5" aria-hidden="true" />
            Save As
          </Button>
          <Button
            size="sm"
            onClick={() => void handleSave()}
            disabled={!activePlan}
            className="h-7 text-xs gap-1.5"
            title="Save plan"
          >
            <Save className="h-3.5 w-3.5" aria-hidden="true" />
            Save
          </Button>
        </div>
      </header>

      {/* Main content */}
      <div className="flex flex-1 min-h-0 overflow-hidden">
        {/* Left panel — plan list */}
        <PlanList />

        {/* Tree + Properties */}
        <div className="flex flex-1 min-h-0 overflow-hidden">
          {/* Tree panel */}
          <div
            className="flex flex-col border-r border-border bg-card overflow-hidden"
            style={{ width: 280, minWidth: 200 }}
          >
            <div className="flex items-center px-3 py-2 border-b border-border shrink-0">
              <span className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                Structure
              </span>
            </div>
            <PlanTree />
          </div>

          {/* Properties panel */}
          <div className="flex flex-col flex-1 min-w-0 overflow-hidden bg-background">
            <div className="flex items-center px-3 py-2 border-b border-border shrink-0">
              <span className="text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                Properties
              </span>
            </div>
            <PlanProperties />
          </div>
        </div>
      </div>
    </div>
  );
}
