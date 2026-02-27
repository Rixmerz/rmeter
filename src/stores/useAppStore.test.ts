import { describe, it, expect, beforeEach } from "vitest";
import { useAppStore } from "./useAppStore";

describe("useAppStore", () => {
  beforeEach(() => {
    // Reset store to initial state between tests
    useAppStore.setState({
      activeView: "request",
      sidebarCollapsed: false,
      theme: "system",
    });
  });

  it("has correct initial state", () => {
    const state = useAppStore.getState();
    expect(state.activeView).toBe("request");
    expect(state.sidebarCollapsed).toBe(false);
    expect(state.theme).toBe("system");
  });

  it("setActiveView changes the active view", () => {
    useAppStore.getState().setActiveView("results");
    expect(useAppStore.getState().activeView).toBe("results");
  });

  it("setActiveView accepts all valid view types", () => {
    const views = ["request", "websocket", "graphql", "test-plans", "results"] as const;
    for (const view of views) {
      useAppStore.getState().setActiveView(view);
      expect(useAppStore.getState().activeView).toBe(view);
    }
  });

  it("toggleSidebar flips collapsed state", () => {
    expect(useAppStore.getState().sidebarCollapsed).toBe(false);
    useAppStore.getState().toggleSidebar();
    expect(useAppStore.getState().sidebarCollapsed).toBe(true);
    useAppStore.getState().toggleSidebar();
    expect(useAppStore.getState().sidebarCollapsed).toBe(false);
  });

  it("setSidebarCollapsed sets explicit value", () => {
    useAppStore.getState().setSidebarCollapsed(true);
    expect(useAppStore.getState().sidebarCollapsed).toBe(true);
    useAppStore.getState().setSidebarCollapsed(false);
    expect(useAppStore.getState().sidebarCollapsed).toBe(false);
  });

  it("setTheme changes the theme", () => {
    useAppStore.getState().setTheme("dark");
    expect(useAppStore.getState().theme).toBe("dark");
    useAppStore.getState().setTheme("light");
    expect(useAppStore.getState().theme).toBe("light");
  });
});
