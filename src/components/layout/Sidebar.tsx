import { Activity, FlaskConical, BarChart3, Zap, Code2 } from "lucide-react";
import { cn } from "@/lib/cn";
import { useAppStore, type ActiveView } from "@/stores/useAppStore";

interface NavItem {
  id: ActiveView;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
}

const navItems: NavItem[] = [
  {
    id: "request",
    label: "Request Builder",
    icon: FlaskConical,
  },
  {
    id: "websocket",
    label: "WebSocket",
    icon: Zap,
  },
  {
    id: "graphql",
    label: "GraphQL",
    icon: Code2,
  },
  {
    id: "test-plans",
    label: "Test Plans",
    icon: Activity,
  },
  {
    id: "results",
    label: "Results",
    icon: BarChart3,
  },
];

export function Sidebar() {
  const { activeView, setActiveView, sidebarCollapsed } = useAppStore();

  return (
    <aside
      className={cn(
        "flex flex-col bg-card border-r border-border transition-all duration-200",
        sidebarCollapsed ? "w-14" : "w-[250px]"
      )}
    >
      <div className="flex items-center h-12 px-4 border-b border-border shrink-0">
        {!sidebarCollapsed && (
          <span className="text-sm font-semibold text-muted-foreground uppercase tracking-wider">
            Navigation
          </span>
        )}
      </div>

      <nav className="flex-1 p-2 space-y-1" aria-label="Main navigation">
        {navItems.map((item) => {
          const Icon = item.icon;
          const isActive = activeView === item.id;

          return (
            <button
              key={item.id}
              onClick={() => setActiveView(item.id)}
              aria-current={isActive ? "page" : undefined}
              className={cn(
                "w-full flex items-center gap-3 px-3 py-2 rounded-md text-sm font-medium transition-colors",
                "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                isActive
                  ? "bg-primary text-primary-foreground"
                  : "text-muted-foreground hover:bg-accent hover:text-accent-foreground"
              )}
            >
              <Icon className="h-4 w-4 shrink-0" />
              {!sidebarCollapsed && (
                <span className="truncate">{item.label}</span>
              )}
            </button>
          );
        })}
      </nav>
    </aside>
  );
}
