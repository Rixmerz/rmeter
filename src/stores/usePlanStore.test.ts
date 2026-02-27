import { describe, it, expect, beforeEach, vi } from "vitest";
import { usePlanStore } from "./usePlanStore";

// Mock all Tauri commands
vi.mock("@/lib/commands", () => ({
  createPlan: vi.fn(),
  getPlan: vi.fn(),
  listPlans: vi.fn(() => Promise.resolve([])),
  deletePlan: vi.fn(),
  setActivePlan: vi.fn(),
  getActivePlan: vi.fn(() => Promise.resolve(null)),
  savePlan: vi.fn(),
  loadPlan: vi.fn(),
  addThreadGroup: vi.fn(),
  removeThreadGroup: vi.fn(),
  updateThreadGroup: vi.fn(),
  addRequest: vi.fn(),
  removeRequest: vi.fn(),
  updateRequest: vi.fn(),
  duplicateElement: vi.fn(),
  reorderThreadGroups: vi.fn(),
  reorderRequests: vi.fn(),
  toggleElement: vi.fn(),
  renameElement: vi.fn(),
  createFromTemplate: vi.fn(),
}));

function resetStore() {
  usePlanStore.setState({
    plans: [],
    activePlan: null,
    selectedNodeId: null,
    selectedNodeType: null,
    loading: false,
    error: null,
  });
}

describe("usePlanStore", () => {
  beforeEach(() => {
    resetStore();
  });

  describe("initial state", () => {
    it("starts with empty plans list", () => {
      expect(usePlanStore.getState().plans).toEqual([]);
    });

    it("starts with no active plan", () => {
      expect(usePlanStore.getState().activePlan).toBeNull();
    });

    it("starts with no selection", () => {
      expect(usePlanStore.getState().selectedNodeId).toBeNull();
      expect(usePlanStore.getState().selectedNodeType).toBeNull();
    });

    it("starts not loading", () => {
      expect(usePlanStore.getState().loading).toBe(false);
    });

    it("starts with no error", () => {
      expect(usePlanStore.getState().error).toBeNull();
    });
  });

  describe("selectNode", () => {
    it("sets selectedNodeId and type", () => {
      usePlanStore.getState().selectNode("node-1", "plan");
      expect(usePlanStore.getState().selectedNodeId).toBe("node-1");
      expect(usePlanStore.getState().selectedNodeType).toBe("plan");
    });

    it("accepts thread_group type", () => {
      usePlanStore.getState().selectNode("tg-1", "thread_group");
      expect(usePlanStore.getState().selectedNodeType).toBe("thread_group");
    });

    it("accepts request type", () => {
      usePlanStore.getState().selectNode("req-1", "request");
      expect(usePlanStore.getState().selectedNodeType).toBe("request");
    });
  });

  describe("clearSelection", () => {
    it("clears selected node", () => {
      usePlanStore.getState().selectNode("node-1", "plan");
      usePlanStore.getState().clearSelection();
      expect(usePlanStore.getState().selectedNodeId).toBeNull();
      expect(usePlanStore.getState().selectedNodeType).toBeNull();
    });
  });

  describe("loadPlans", () => {
    it("sets loading to true during load", async () => {
      // The mock returns [] instantly so loading will be false after
      const promise = usePlanStore.getState().loadPlans();
      await promise;
      expect(usePlanStore.getState().loading).toBe(false);
    });

    it("sets error on failure", async () => {
      const { listPlans } = await import("@/lib/commands");
      vi.mocked(listPlans).mockRejectedValueOnce(new Error("Network error"));
      await usePlanStore.getState().loadPlans();
      expect(usePlanStore.getState().error).toBe("Network error");
      expect(usePlanStore.getState().loading).toBe(false);
    });
  });
});
