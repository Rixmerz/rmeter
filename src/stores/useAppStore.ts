import { create } from "zustand";

export type ActiveView = "request" | "websocket" | "graphql" | "test-plans" | "results";
export type Theme = "light" | "dark" | "system";

interface AppState {
  activeView: ActiveView;
  sidebarCollapsed: boolean;
  theme: Theme;
}

interface AppActions {
  setActiveView: (view: ActiveView) => void;
  toggleSidebar: () => void;
  setSidebarCollapsed: (collapsed: boolean) => void;
  setTheme: (theme: Theme) => void;
}

export const useAppStore = create<AppState & AppActions>((set) => ({
  activeView: "request",
  sidebarCollapsed: false,
  theme: "system",

  setActiveView: (view) => set({ activeView: view }),
  toggleSidebar: () =>
    set((state) => ({ sidebarCollapsed: !state.sidebarCollapsed })),
  setSidebarCollapsed: (collapsed) => set({ sidebarCollapsed: collapsed }),
  setTheme: (theme) => set({ theme }),
}));
