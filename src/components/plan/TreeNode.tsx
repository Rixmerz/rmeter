import { useState, useRef } from "react";
import {
  ChevronRight,
  ChevronDown,
  Folder,
  FolderOpen,
  Globe,
  Copy,
  Trash2,
  Pencil,
  ToggleLeft,
  ToggleRight,
  Plus,
} from "lucide-react";
import { cn } from "@/lib/cn";
import { usePlanStore } from "@/stores/usePlanStore";
import type { ThreadGroup, HttpRequest } from "@/types/plan";

// ----------------------------------------------------------------
// Context menu
// ----------------------------------------------------------------

interface ContextMenuProps {
  x: number;
  y: number;
  onRename: () => void;
  onDuplicate: () => void;
  onToggle: () => void;
  onDelete: () => void;
  onAddRequest?: () => void;
  onClose: () => void;
  isEnabled: boolean;
}

function ContextMenu({
  x,
  y,
  onRename,
  onDuplicate,
  onToggle,
  onDelete,
  onAddRequest,
  onClose,
  isEnabled,
}: ContextMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null);

  function handleItemClick(action: () => void) {
    action();
    onClose();
  }

  return (
    <>
      {/* Invisible overlay to catch outside clicks */}
      <div className="fixed inset-0 z-40" onClick={onClose} />
      <div
        ref={menuRef}
        role="menu"
        aria-label="Node actions"
        className={cn(
          "fixed z-50 min-w-[160px] rounded-md border border-border bg-popover shadow-md py-1",
          "text-sm text-popover-foreground"
        )}
        style={{ left: x, top: y }}
      >
        {onAddRequest && (
          <>
            <button
              role="menuitem"
              onClick={() => handleItemClick(onAddRequest)}
              className="flex w-full items-center gap-2 px-3 py-1.5 hover:bg-accent hover:text-accent-foreground"
            >
              <Plus className="h-3.5 w-3.5" aria-hidden="true" />
              Add Request
            </button>
            <div className="my-1 border-t border-border" />
          </>
        )}
        <button
          role="menuitem"
          onClick={() => handleItemClick(onRename)}
          className="flex w-full items-center gap-2 px-3 py-1.5 hover:bg-accent hover:text-accent-foreground"
        >
          <Pencil className="h-3.5 w-3.5" aria-hidden="true" />
          Rename
        </button>
        <button
          role="menuitem"
          onClick={() => handleItemClick(onDuplicate)}
          className="flex w-full items-center gap-2 px-3 py-1.5 hover:bg-accent hover:text-accent-foreground"
        >
          <Copy className="h-3.5 w-3.5" aria-hidden="true" />
          Duplicate
        </button>
        <button
          role="menuitem"
          onClick={() => handleItemClick(onToggle)}
          className="flex w-full items-center gap-2 px-3 py-1.5 hover:bg-accent hover:text-accent-foreground"
        >
          {isEnabled ? (
            <ToggleRight className="h-3.5 w-3.5" aria-hidden="true" />
          ) : (
            <ToggleLeft className="h-3.5 w-3.5" aria-hidden="true" />
          )}
          {isEnabled ? "Disable" : "Enable"}
        </button>
        <div className="my-1 border-t border-border" />
        <button
          role="menuitem"
          onClick={() => handleItemClick(onDelete)}
          className="flex w-full items-center gap-2 px-3 py-1.5 text-destructive hover:bg-destructive/10"
        >
          <Trash2 className="h-3.5 w-3.5" aria-hidden="true" />
          Delete
        </button>
      </div>
    </>
  );
}

// ----------------------------------------------------------------
// Inline rename
// ----------------------------------------------------------------

interface InlineRenameProps {
  initialName: string;
  onCommit: (name: string) => void;
  onCancel: () => void;
}

function InlineRename({ initialName, onCommit, onCancel }: InlineRenameProps) {
  const [value, setValue] = useState(initialName);

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Enter") {
      const trimmed = value.trim();
      if (trimmed) onCommit(trimmed);
      else onCancel();
    } else if (e.key === "Escape") {
      onCancel();
    }
  }

  return (
    <input
      autoFocus
      type="text"
      value={value}
      onChange={(e) => setValue(e.target.value)}
      onBlur={() => {
        const trimmed = value.trim();
        if (trimmed) onCommit(trimmed);
        else onCancel();
      }}
      onKeyDown={handleKeyDown}
      aria-label="Rename"
      className={cn(
        "flex-1 min-w-0 text-sm px-1 py-0 rounded border border-ring bg-background",
        "focus:outline-none focus:ring-1 focus:ring-ring"
      )}
      onClick={(e) => e.stopPropagation()}
    />
  );
}

// ----------------------------------------------------------------
// RequestNode
// ----------------------------------------------------------------

interface RequestNodeProps {
  request: HttpRequest;
  groupId: string;
  depth: number;
}

const METHOD_COLORS: Record<string, string> = {
  GET: "text-green-600 dark:text-green-400",
  POST: "text-blue-600 dark:text-blue-400",
  PUT: "text-yellow-600 dark:text-yellow-400",
  DELETE: "text-red-600 dark:text-red-400",
  PATCH: "text-orange-600 dark:text-orange-400",
  HEAD: "text-purple-600 dark:text-purple-400",
  OPTIONS: "text-cyan-600 dark:text-cyan-400",
};

export function RequestNode({ request, groupId: _groupId, depth }: RequestNodeProps) {
  const { selectedNodeId, selectNode, removeElement, duplicateElement, toggleElement, renameElement } =
    usePlanStore();
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(null);
  const [renaming, setRenaming] = useState(false);

  const isSelected = selectedNodeId === request.id;

  function handleContextMenu(e: React.MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({ x: e.clientX, y: e.clientY });
  }

  function handleRenameCommit(name: string) {
    setRenaming(false);
    void renameElement(request.id, name);
  }

  return (
    <>
      <div
        role="treeitem"
        aria-selected={isSelected}
        tabIndex={0}
        style={{ paddingLeft: depth * 16 + 8 }}
        className={cn(
          "flex items-center gap-1.5 py-1.5 pr-2 rounded-md cursor-pointer select-none",
          "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
          !request.enabled && "opacity-50",
          isSelected
            ? "bg-primary text-primary-foreground"
            : "hover:bg-accent hover:text-accent-foreground"
        )}
        onClick={() => selectNode(request.id, "request")}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") selectNode(request.id, "request");
        }}
        onContextMenu={handleContextMenu}
      >
        <Globe
          className="h-3.5 w-3.5 shrink-0"
          aria-hidden="true"
        />
        <span
          className={cn(
            "text-xs font-mono font-bold shrink-0",
            isSelected ? "text-primary-foreground/80" : (METHOD_COLORS[request.method] ?? "text-muted-foreground")
          )}
        >
          {request.method}
        </span>
        {renaming ? (
          <InlineRename
            initialName={request.name}
            onCommit={handleRenameCommit}
            onCancel={() => setRenaming(false)}
          />
        ) : (
          <span className="text-sm truncate flex-1">{request.name}</span>
        )}
      </div>

      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          isEnabled={request.enabled}
          onRename={() => setRenaming(true)}
          onDuplicate={() => void duplicateElement(request.id)}
          onToggle={() => void toggleElement(request.id)}
          onDelete={() => void removeElement(request.id)}
          onClose={() => setContextMenu(null)}
        />
      )}
    </>
  );
}

// ----------------------------------------------------------------
// ThreadGroupNode
// ----------------------------------------------------------------

interface ThreadGroupNodeProps {
  group: ThreadGroup;
  depth: number;
}

export function ThreadGroupNode({ group, depth }: ThreadGroupNodeProps) {
  const {
    selectedNodeId,
    selectNode,
    removeElement,
    duplicateElement,
    toggleElement,
    renameElement,
    addRequest,
  } = usePlanStore();
  const [expanded, setExpanded] = useState(true);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(null);
  const [renaming, setRenaming] = useState(false);

  const isSelected = selectedNodeId === group.id;

  function handleContextMenu(e: React.MouseEvent) {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({ x: e.clientX, y: e.clientY });
  }

  function handleAddRequest() {
    void addRequest(group.id, "New Request");
    setExpanded(true);
  }

  function handleRenameCommit(name: string) {
    setRenaming(false);
    void renameElement(group.id, name);
  }

  const ChevronIcon = expanded ? ChevronDown : ChevronRight;
  const FolderIcon = expanded ? FolderOpen : Folder;

  return (
    <>
      <div
        role="treeitem"
        aria-selected={isSelected}
        aria-expanded={expanded}
        tabIndex={0}
        style={{ paddingLeft: depth * 16 + 4 }}
        className={cn(
          "flex items-center gap-1.5 py-1.5 pr-2 rounded-md cursor-pointer select-none",
          "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
          !group.enabled && "opacity-50",
          isSelected
            ? "bg-primary text-primary-foreground"
            : "hover:bg-accent hover:text-accent-foreground"
        )}
        onClick={() => selectNode(group.id, "thread_group")}
        onKeyDown={(e) => {
          if (e.key === "Enter") selectNode(group.id, "thread_group");
          if (e.key === " ") { e.preventDefault(); setExpanded((v) => !v); }
          if (e.key === "ArrowRight") setExpanded(true);
          if (e.key === "ArrowLeft") setExpanded(false);
        }}
        onContextMenu={handleContextMenu}
      >
        <button
          aria-label={expanded ? "Collapse" : "Expand"}
          onClick={(e) => { e.stopPropagation(); setExpanded((v) => !v); }}
          className="shrink-0 rounded hover:bg-accent/50 p-0.5 -ml-0.5"
          tabIndex={-1}
        >
          <ChevronIcon className="h-3.5 w-3.5" aria-hidden="true" />
        </button>
        <FolderIcon className="h-3.5 w-3.5 shrink-0" aria-hidden="true" />
        {renaming ? (
          <InlineRename
            initialName={group.name}
            onCommit={handleRenameCommit}
            onCancel={() => setRenaming(false)}
          />
        ) : (
          <span className="text-sm font-medium truncate flex-1">{group.name}</span>
        )}
        <span
          className={cn(
            "text-xs shrink-0",
            isSelected ? "text-primary-foreground/60" : "text-muted-foreground"
          )}
        >
          {group.num_threads}t
        </span>
      </div>

      {/* Children */}
      {expanded && (
        <div role="group">
          {group.requests.map((req) => (
            <RequestNode key={req.id} request={req} groupId={group.id} depth={depth + 1} />
          ))}
          {group.requests.length === 0 && (
            <div
              style={{ paddingLeft: (depth + 1) * 16 + 8 }}
              className="py-1 text-xs text-muted-foreground italic select-none"
            >
              No requests
            </div>
          )}
        </div>
      )}

      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          isEnabled={group.enabled}
          onRename={() => setRenaming(true)}
          onDuplicate={() => void duplicateElement(group.id)}
          onToggle={() => void toggleElement(group.id)}
          onDelete={() => void removeElement(group.id)}
          onAddRequest={handleAddRequest}
          onClose={() => setContextMenu(null)}
        />
      )}
    </>
  );
}
