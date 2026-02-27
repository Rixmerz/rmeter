import { cn } from "@/lib/cn";
import { Button } from "@/components/ui/button";
import { X, Globe, Zap, AlertTriangle } from "lucide-react";

interface Template {
  id: string;
  label: string;
  description: string;
  icon: React.ComponentType<{ className?: string }>;
}

const TEMPLATES: Template[] = [
  {
    id: "rest_api",
    label: "REST API",
    description: "A simple REST API test plan with one thread group and sample requests.",
    icon: Globe,
  },
  {
    id: "load_test",
    label: "Load Test",
    description: "Ramp-up load test with configurable virtual users and hold duration.",
    icon: Zap,
  },
  {
    id: "stress_test",
    label: "Stress Test",
    description: "High-concurrency stress test to find system breaking points.",
    icon: AlertTriangle,
  },
];

interface TemplateDialogProps {
  open: boolean;
  onClose: () => void;
  onSelect: (templateId: string) => void;
}

export function TemplateDialog({ open, onClose, onSelect }: TemplateDialogProps) {
  if (!open) return null;

  function handleBackdropClick(e: React.MouseEvent<HTMLDivElement>) {
    if (e.target === e.currentTarget) {
      onClose();
    }
  }

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Escape") {
      onClose();
    }
  }

  return (
    <div
      role="dialog"
      aria-modal="true"
      aria-label="Choose a template"
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      onClick={handleBackdropClick}
      onKeyDown={handleKeyDown}
    >
      <div className="bg-card border border-border rounded-lg shadow-lg w-96 max-w-full mx-4">
        {/* Header */}
        <div className="flex items-center justify-between px-4 py-3 border-b border-border">
          <h2 className="text-sm font-semibold">Choose a Template</h2>
          <button
            onClick={onClose}
            aria-label="Close dialog"
            className={cn(
              "rounded p-1 hover:bg-accent hover:text-accent-foreground",
              "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
            )}
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        {/* Template list */}
        <div className="p-4 space-y-3">
          {TEMPLATES.map((tpl) => {
            const Icon = tpl.icon;
            return (
              <button
                key={tpl.id}
                onClick={() => onSelect(tpl.id)}
                className={cn(
                  "w-full text-left flex items-start gap-3 p-3 rounded-md border border-border",
                  "hover:bg-accent hover:text-accent-foreground hover:border-accent",
                  "focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring",
                  "transition-colors"
                )}
              >
                <Icon className="h-5 w-5 shrink-0 mt-0.5 text-muted-foreground" aria-hidden="true" />
                <div className="min-w-0">
                  <div className="text-sm font-medium">{tpl.label}</div>
                  <div className="text-xs text-muted-foreground mt-0.5">{tpl.description}</div>
                </div>
              </button>
            );
          })}
        </div>

        {/* Footer */}
        <div className="flex justify-end px-4 pb-4">
          <Button variant="outline" size="sm" onClick={onClose}>
            Cancel
          </Button>
        </div>
      </div>
    </div>
  );
}
