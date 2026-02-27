import { PanelLeftClose, PanelLeftOpen } from "lucide-react";
import { Sidebar } from "./Sidebar";
import { Button } from "@/components/ui/button";
import { useAppStore } from "@/stores/useAppStore";
import { useEngineStore } from "@/stores/useEngineStore";
import { EngineControlBar } from "@/components/engine/EngineControlBar";

interface AppLayoutProps {
  children: React.ReactNode;
}

export function AppLayout({ children }: AppLayoutProps) {
  const { toggleSidebar, sidebarCollapsed } = useAppStore();
  const engineStatus = useEngineStore((s) => s.status);

  const showEngineBar = engineStatus !== "idle";

  return (
    <div className="flex h-screen w-screen overflow-hidden bg-background">
      <Sidebar />

      <div className="flex flex-col flex-1 min-w-0">
        {/* Header */}
        <header className="flex items-center h-12 px-4 border-b border-border bg-card shrink-0 gap-3">
          <Button
            variant="ghost"
            size="icon"
            onClick={toggleSidebar}
            aria-label={sidebarCollapsed ? "Expand sidebar" : "Collapse sidebar"}
            className="h-8 w-8"
          >
            {sidebarCollapsed ? (
              <PanelLeftOpen className="h-4 w-4" />
            ) : (
              <PanelLeftClose className="h-4 w-4" />
            )}
          </Button>

          <div className="flex items-center gap-2">
            <span className="text-lg font-bold tracking-tight">rmeter</span>
            <span className="text-xs text-muted-foreground bg-muted px-1.5 py-0.5 rounded">
              v0.1.0
            </span>
          </div>

          <div className="ml-auto" />
        </header>

        {/* Engine control bar — visible when a test is active */}
        {showEngineBar && <EngineControlBar />}

        {/* Main content */}
        <main className="flex-1 min-h-0 overflow-auto" role="main">
          {children}
        </main>
      </div>
    </div>
  );
}
