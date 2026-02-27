import { create } from "zustand";
import type {
  TestPlan,
  PlanSummary,
  ThreadGroupUpdate,
  HttpRequestUpdate,
} from "@/types/plan";
import {
  createPlan as createPlanCmd,
  getPlan,
  listPlans as listPlansCmd,
  deletePlan as deletePlanCmd,
  setActivePlan,
  getActivePlan,
  savePlan as savePlanCmd,
  loadPlan as loadPlanCmd,
  addThreadGroup as addThreadGroupCmd,
  removeThreadGroup as removeThreadGroupCmd,
  updateThreadGroup as updateThreadGroupCmd,
  addRequest as addRequestCmd,
  removeRequest as removeRequestCmd,
  updateRequest as updateRequestCmd,
  duplicateElement as duplicateElementCmd,
  reorderThreadGroups as reorderThreadGroupsCmd,
  reorderRequests as reorderRequestsCmd,
  toggleElement as toggleElementCmd,
  renameElement as renameElementCmd,
  createFromTemplate as createFromTemplateCmd,
} from "@/lib/commands";

export type NodeType = "plan" | "thread_group" | "request";

interface PlanState {
  plans: PlanSummary[];
  activePlan: TestPlan | null;
  selectedNodeId: string | null;
  selectedNodeType: NodeType | null;
  loading: boolean;
  error: string | null;
}

interface PlanActions {
  // Plan CRUD
  loadPlans(): Promise<void>;
  createPlan(name: string): Promise<void>;
  createFromTemplate(template: string): Promise<void>;
  selectPlan(id: string): Promise<void>;
  deletePlan(id: string): Promise<void>;

  // File I/O
  savePlan(path: string): Promise<void>;
  loadPlanFromFile(path: string): Promise<void>;

  // Tree operations
  selectNode(id: string, type: NodeType): void;
  clearSelection(): void;
  addThreadGroup(name: string): Promise<void>;
  addRequest(groupId: string, name: string): Promise<void>;
  removeElement(elementId: string): Promise<void>;
  duplicateElement(elementId: string): Promise<void>;
  toggleElement(elementId: string): Promise<void>;
  renameElement(elementId: string, newName: string): Promise<void>;
  reorderThreadGroups(groupIds: string[]): Promise<void>;
  reorderRequests(groupId: string, requestIds: string[]): Promise<void>;

  // Thread group / request editing
  updateThreadGroup(groupId: string, update: ThreadGroupUpdate): Promise<void>;
  updateRequest(
    groupId: string,
    requestId: string,
    update: HttpRequestUpdate
  ): Promise<void>;
}

export const usePlanStore = create<PlanState & PlanActions>((set, get) => ({
  plans: [],
  activePlan: null,
  selectedNodeId: null,
  selectedNodeType: null,
  loading: false,
  error: null,

  loadPlans: async () => {
    set({ loading: true, error: null });
    try {
      const [plans, activePlan] = await Promise.all([
        listPlansCmd(),
        getActivePlan(),
      ]);
      set({ plans, activePlan, loading: false });
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : String(err),
        loading: false,
      });
    }
  },

  createPlan: async (name) => {
    set({ loading: true, error: null });
    try {
      const plan = await createPlanCmd(name);
      await setActivePlan(plan.id);
      const plans = await listPlansCmd();
      set({ plans, activePlan: plan, selectedNodeId: plan.id, selectedNodeType: "plan", loading: false });
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : String(err),
        loading: false,
      });
    }
  },

  createFromTemplate: async (template) => {
    set({ loading: true, error: null });
    try {
      const plan = await createFromTemplateCmd(template);
      await setActivePlan(plan.id);
      const plans = await listPlansCmd();
      set({ plans, activePlan: plan, selectedNodeId: plan.id, selectedNodeType: "plan", loading: false });
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : String(err),
        loading: false,
      });
    }
  },

  selectPlan: async (id) => {
    set({ loading: true, error: null });
    try {
      await setActivePlan(id);
      const plan = await getPlan(id);
      set({ activePlan: plan, selectedNodeId: id, selectedNodeType: "plan", loading: false });
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : String(err),
        loading: false,
      });
    }
  },

  deletePlan: async (id) => {
    set({ loading: true, error: null });
    try {
      await deletePlanCmd(id);
      const { activePlan } = get();
      const plans = await listPlansCmd();
      let newActivePlan: TestPlan | null = null;
      if (activePlan?.id === id) {
        if (plans.length > 0) {
          await setActivePlan(plans[0].id);
          newActivePlan = await getPlan(plans[0].id);
        }
        set({
          plans,
          activePlan: newActivePlan,
          selectedNodeId: null,
          selectedNodeType: null,
          loading: false,
        });
      } else {
        set({ plans, loading: false });
      }
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : String(err),
        loading: false,
      });
    }
  },

  savePlan: async (path) => {
    const { activePlan } = get();
    if (!activePlan) return;
    set({ loading: true, error: null });
    try {
      await savePlanCmd(activePlan.id, path);
      set({ loading: false });
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : String(err),
        loading: false,
      });
    }
  },

  loadPlanFromFile: async (path) => {
    set({ loading: true, error: null });
    try {
      const plan = await loadPlanCmd(path);
      await setActivePlan(plan.id);
      const plans = await listPlansCmd();
      set({ plans, activePlan: plan, selectedNodeId: plan.id, selectedNodeType: "plan", loading: false });
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : String(err),
        loading: false,
      });
    }
  },

  selectNode: (id, type) => {
    set({ selectedNodeId: id, selectedNodeType: type });
  },

  clearSelection: () => {
    set({ selectedNodeId: null, selectedNodeType: null });
  },

  addThreadGroup: async (name) => {
    const { activePlan } = get();
    if (!activePlan) return;
    set({ error: null });
    try {
      const updatedPlan = await getPlan(activePlan.id);
      await addThreadGroupCmd(activePlan.id, name);
      const freshPlan = await getPlan(activePlan.id);
      // Select the newly added group
      const newGroup = freshPlan.thread_groups.find(
        (g) => !updatedPlan.thread_groups.some((og) => og.id === g.id)
      );
      set({
        activePlan: freshPlan,
        selectedNodeId: newGroup?.id ?? null,
        selectedNodeType: newGroup ? "thread_group" : null,
      });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  addRequest: async (groupId, name) => {
    const { activePlan } = get();
    if (!activePlan) return;
    set({ error: null });
    try {
      const group = activePlan.thread_groups.find((g) => g.id === groupId);
      await addRequestCmd(activePlan.id, groupId, name);
      const freshPlan = await getPlan(activePlan.id);
      const freshGroup = freshPlan.thread_groups.find((g) => g.id === groupId);
      // Select the newly added request
      const newRequest = freshGroup?.requests.find(
        (r) => !group?.requests.some((or) => or.id === r.id)
      );
      set({
        activePlan: freshPlan,
        selectedNodeId: newRequest?.id ?? null,
        selectedNodeType: newRequest ? "request" : null,
      });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  removeElement: async (elementId) => {
    const { activePlan } = get();
    if (!activePlan) return;
    set({ error: null });
    try {
      // Check if it's a thread group or a request
      const isGroup = activePlan.thread_groups.some((g) => g.id === elementId);
      if (isGroup) {
        await removeThreadGroupCmd(activePlan.id, elementId);
      } else {
        // Find which group owns this request
        const ownerGroup = activePlan.thread_groups.find((g) =>
          g.requests.some((r) => r.id === elementId)
        );
        if (ownerGroup) {
          await removeRequestCmd(activePlan.id, ownerGroup.id, elementId);
        }
      }
      const freshPlan = await getPlan(activePlan.id);
      set({
        activePlan: freshPlan,
        selectedNodeId: null,
        selectedNodeType: null,
      });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  duplicateElement: async (elementId) => {
    const { activePlan } = get();
    if (!activePlan) return;
    set({ error: null });
    try {
      const updatedPlan = await duplicateElementCmd(activePlan.id, elementId);
      set({ activePlan: updatedPlan });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  toggleElement: async (elementId) => {
    const { activePlan } = get();
    if (!activePlan) return;
    set({ error: null });
    try {
      await toggleElementCmd(activePlan.id, elementId);
      const freshPlan = await getPlan(activePlan.id);
      set({ activePlan: freshPlan });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  renameElement: async (elementId, newName) => {
    const { activePlan } = get();
    if (!activePlan) return;
    set({ error: null });
    try {
      await renameElementCmd(activePlan.id, elementId, newName);
      const freshPlan = await getPlan(activePlan.id);
      set({ activePlan: freshPlan });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  reorderThreadGroups: async (groupIds) => {
    const { activePlan } = get();
    if (!activePlan) return;
    set({ error: null });
    try {
      await reorderThreadGroupsCmd(activePlan.id, groupIds);
      const freshPlan = await getPlan(activePlan.id);
      set({ activePlan: freshPlan });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  reorderRequests: async (groupId, requestIds) => {
    const { activePlan } = get();
    if (!activePlan) return;
    set({ error: null });
    try {
      await reorderRequestsCmd(activePlan.id, groupId, requestIds);
      const freshPlan = await getPlan(activePlan.id);
      set({ activePlan: freshPlan });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  updateThreadGroup: async (groupId, update) => {
    const { activePlan } = get();
    if (!activePlan) return;
    set({ error: null });
    try {
      await updateThreadGroupCmd(activePlan.id, groupId, update);
      const freshPlan = await getPlan(activePlan.id);
      set({ activePlan: freshPlan });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  updateRequest: async (groupId, requestId, update) => {
    const { activePlan } = get();
    if (!activePlan) return;
    set({ error: null });
    try {
      await updateRequestCmd(activePlan.id, groupId, requestId, update);
      const freshPlan = await getPlan(activePlan.id);
      set({ activePlan: freshPlan });
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },
}));
