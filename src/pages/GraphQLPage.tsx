import { useState } from "react";
import { Plus, X, Loader2, Search, Send, Code2, ChevronDown, ChevronRight } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/cn";
import { sendGraphql, graphqlIntrospect } from "@/lib/commands";
import type { SendRequestOutput } from "@/types/request";

// ----------------------------------------------------------------
// Helpers / shared styling
// ----------------------------------------------------------------

function inputClass(extra?: string) {
  return cn(
    "h-8 px-3 rounded-md border border-input bg-background text-sm",
    "placeholder:text-muted-foreground",
    "focus:outline-none focus:ring-1 focus:ring-ring",
    extra
  );
}

function getStatusColor(status: number): string {
  if (status >= 200 && status < 300) return "text-green-600 dark:text-green-400";
  if (status >= 300 && status < 400) return "text-yellow-600 dark:text-yellow-400";
  if (status >= 400 && status < 500) return "text-red-600 dark:text-red-400";
  if (status >= 500) return "text-red-700 dark:text-red-300";
  return "text-muted-foreground";
}

// ----------------------------------------------------------------
// Header editor (local to this page)
// ----------------------------------------------------------------

interface HeaderEntry {
  id: string;
  key: string;
  value: string;
}

interface HeaderEditorProps {
  headers: HeaderEntry[];
  onChange: (headers: HeaderEntry[]) => void;
}

function HeaderEditor({ headers, onChange }: HeaderEditorProps) {
  const addRow = () =>
    onChange([...headers, { id: crypto.randomUUID(), key: "", value: "" }]);
  const updateRow = (id: string, key: string, value: string) =>
    onChange(headers.map((h) => (h.id === id ? { ...h, key, value } : h)));
  const removeRow = (id: string) => onChange(headers.filter((h) => h.id !== id));

  return (
    <div className="rounded-lg border border-border bg-card">
      <div className="flex items-center justify-between px-4 py-3 border-b border-border">
        <h2 className="text-sm font-semibold">
          Headers{" "}
          {headers.length > 0 && (
            <span className="text-xs text-muted-foreground font-normal">
              ({headers.length})
            </span>
          )}
        </h2>
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={addRow}
          className="h-7 gap-1 text-xs"
        >
          <Plus className="h-3 w-3" />
          Add Header
        </Button>
      </div>
      <div className="p-3 space-y-2">
        {headers.length === 0 ? (
          <p className="text-sm text-muted-foreground text-center py-3">
            No headers. Click "Add Header" to add one.
          </p>
        ) : (
          headers.map((h) => (
            <div key={h.id} className="flex gap-2 items-center">
              <input
                type="text"
                value={h.key}
                onChange={(e) => updateRow(h.id, e.target.value, h.value)}
                placeholder="Header name"
                aria-label="Header name"
                className={inputClass("flex-1")}
              />
              <input
                type="text"
                value={h.value}
                onChange={(e) => updateRow(h.id, h.key, e.target.value)}
                placeholder="Header value"
                aria-label="Header value"
                className={inputClass("flex-1")}
              />
              <Button
                type="button"
                variant="ghost"
                size="icon"
                onClick={() => removeRow(h.id)}
                aria-label="Remove header"
                className="h-8 w-8 text-muted-foreground hover:text-destructive"
              >
                <X className="h-3.5 w-3.5" />
              </Button>
            </div>
          ))
        )}
      </div>
    </div>
  );
}

// ----------------------------------------------------------------
// Collapsible section
// ----------------------------------------------------------------

interface CollapsibleSectionProps {
  title: string;
  children: React.ReactNode;
  defaultOpen?: boolean;
  action?: React.ReactNode;
}

function CollapsibleSection({
  title,
  children,
  defaultOpen = true,
  action,
}: CollapsibleSectionProps) {
  const [open, setOpen] = useState(defaultOpen);
  return (
    <div className="rounded-lg border border-border bg-card">
      <div className="flex items-center justify-between px-4 py-3 border-b border-border">
        <button
          type="button"
          onClick={() => setOpen((v) => !v)}
          className="flex items-center gap-2 text-sm font-semibold hover:text-foreground/80 transition-colors"
          aria-expanded={open}
        >
          {open ? (
            <ChevronDown className="h-4 w-4 text-muted-foreground" />
          ) : (
            <ChevronRight className="h-4 w-4 text-muted-foreground" />
          )}
          {title}
        </button>
        {action}
      </div>
      {open && children}
    </div>
  );
}

// ----------------------------------------------------------------
// Response viewer (reused from RequestPage pattern)
// ----------------------------------------------------------------

interface ResponseViewerProps {
  response: SendRequestOutput;
}

function ResponseViewer({ response }: ResponseViewerProps) {
  const [activeTab, setActiveTab] = useState<"body" | "headers">("body");

  const formattedBody = (() => {
    try {
      return JSON.stringify(JSON.parse(response.body), null, 2);
    } catch {
      return response.body;
    }
  })();

  return (
    <div className="rounded-lg border border-border bg-card flex flex-col">
      {/* Status bar */}
      <div className="flex items-center gap-4 px-4 py-3 border-b border-border">
        <h2 className="text-sm font-semibold">Response</h2>
        <div className="flex items-center gap-3 ml-auto text-xs text-muted-foreground">
          <span
            className={cn("font-semibold text-sm", getStatusColor(response.status))}
            aria-label={`Status code ${response.status}`}
          >
            {response.status}
          </span>
          <span>{response.elapsed_ms}ms</span>
          <span>{(response.size_bytes / 1024).toFixed(1)} KB</span>
        </div>
      </div>

      {/* Tab switcher */}
      <div className="flex gap-1 px-4 pt-3 border-b border-border">
        {(["body", "headers"] as const).map((tab) => (
          <button
            key={tab}
            type="button"
            onClick={() => setActiveTab(tab)}
            className={cn(
              "px-3 py-1.5 text-xs font-medium rounded-t transition-colors capitalize",
              activeTab === tab
                ? "bg-primary text-primary-foreground"
                : "text-muted-foreground hover:text-foreground hover:bg-accent"
            )}
            aria-selected={activeTab === tab}
          >
            {tab}
          </button>
        ))}
      </div>

      {/* Tab content */}
      <div className="p-4">
        {activeTab === "body" && (
          <pre className="text-sm font-mono bg-muted/50 rounded-md p-3 overflow-auto max-h-80 whitespace-pre-wrap break-all">
            {formattedBody}
          </pre>
        )}
        {activeTab === "headers" && (
          <div className="space-y-1">
            {Object.entries(response.headers).map(([key, value]) => (
              <div key={key} className="flex gap-2 text-xs font-mono">
                <span className="text-muted-foreground min-w-[180px] truncate">{key}</span>
                <span className="text-foreground break-all">{value}</span>
              </div>
            ))}
            {Object.keys(response.headers).length === 0 && (
              <p className="text-xs text-muted-foreground italic">No response headers.</p>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

// ----------------------------------------------------------------
// Schema tree node (for introspection result display)
// ----------------------------------------------------------------

interface SchemaNodeProps {
  label: string;
  value: unknown;
  depth?: number;
}

function SchemaNode({ label, value, depth = 0 }: SchemaNodeProps) {
  const [open, setOpen] = useState(depth < 2);
  const isObject = value !== null && typeof value === "object" && !Array.isArray(value);
  const isArray = Array.isArray(value);
  const hasChildren = isObject || isArray;
  const childEntries = isObject
    ? Object.entries(value as Record<string, unknown>)
    : isArray
    ? (value as unknown[]).map((v, i) => [String(i), v] as [string, unknown])
    : [];

  const primitiveStr =
    value === null
      ? "null"
      : typeof value === "string"
      ? `"${value}"`
      : String(value);

  return (
    <div style={{ paddingLeft: depth * 12 }}>
      {hasChildren ? (
        <>
          <button
            type="button"
            onClick={() => setOpen((v) => !v)}
            className="flex items-center gap-1 text-xs text-left w-full hover:text-foreground text-muted-foreground transition-colors"
            aria-expanded={open}
          >
            {open ? (
              <ChevronDown className="h-3 w-3 shrink-0" />
            ) : (
              <ChevronRight className="h-3 w-3 shrink-0" />
            )}
            <span className="font-medium text-foreground">{label}</span>
            <span className="text-muted-foreground/60 ml-1">
              {isArray ? `[${(value as unknown[]).length}]` : `{${childEntries.length}}`}
            </span>
          </button>
          {open && (
            <div className="mt-0.5">
              {childEntries.map(([k, v]) => (
                <SchemaNode key={k} label={k} value={v} depth={depth + 1} />
              ))}
            </div>
          )}
        </>
      ) : (
        <div className="flex items-baseline gap-1 text-xs py-0.5 pl-4">
          <span className="font-medium text-foreground">{label}:</span>
          <span className="text-muted-foreground font-mono">{primitiveStr}</span>
        </div>
      )}
    </div>
  );
}

// ----------------------------------------------------------------
// Main page
// ----------------------------------------------------------------

export function GraphQLPage() {
  const [endpointUrl, setEndpointUrl] = useState("https://");
  const [headers, setHeaders] = useState<HeaderEntry[]>([]);
  const [query, setQuery] = useState(
    "query {\n  # Write your GraphQL query here\n}"
  );
  const [variables, setVariables] = useState("");
  const [operationName, setOperationName] = useState("");

  const [sendLoading, setSendLoading] = useState(false);
  const [sendError, setSendError] = useState<string | null>(null);
  const [response, setResponse] = useState<SendRequestOutput | null>(null);

  const [introspectLoading, setIntrospectLoading] = useState(false);
  const [introspectError, setIntrospectError] = useState<string | null>(null);
  const [schema, setSchema] = useState<unknown | null>(null);

  // ---- helpers ----
  const buildHeaders = () =>
    Object.fromEntries(
      headers
        .filter((h) => h.key.trim() !== "")
        .map((h) => [h.key.trim(), h.value.trim()])
    );

  const parseVariables = (): { ok: true; value: unknown } | { ok: false; error: string } => {
    const trimmed = variables.trim();
    if (!trimmed) return { ok: true, value: undefined };
    try {
      return { ok: true, value: JSON.parse(trimmed) };
    } catch {
      return { ok: false, error: "Variables must be valid JSON." };
    }
  };

  // ---- send ----
  const handleSend = async () => {
    if (!endpointUrl.trim()) {
      setSendError("Endpoint URL is required.");
      return;
    }
    if (!query.trim()) {
      setSendError("Query is required.");
      return;
    }

    const varsResult = parseVariables();
    if (!varsResult.ok) {
      setSendError(varsResult.error);
      return;
    }

    setSendLoading(true);
    setSendError(null);
    setResponse(null);

    try {
      const res = await sendGraphql(
        endpointUrl.trim(),
        query.trim(),
        varsResult.value,
        operationName.trim() || undefined,
        buildHeaders()
      );
      setResponse(res);
    } catch (err) {
      setSendError(err instanceof Error ? err.message : String(err));
    } finally {
      setSendLoading(false);
    }
  };

  // ---- introspect ----
  const handleIntrospect = async () => {
    if (!endpointUrl.trim()) {
      setIntrospectError("Endpoint URL is required.");
      return;
    }

    setIntrospectLoading(true);
    setIntrospectError(null);
    setSchema(null);

    try {
      const result = await graphqlIntrospect(endpointUrl.trim(), buildHeaders());
      setSchema(result);
    } catch (err) {
      setIntrospectError(err instanceof Error ? err.message : String(err));
    } finally {
      setIntrospectLoading(false);
    }
  };

  return (
    <div className="max-w-5xl mx-auto space-y-5 p-6">
      {/* Page header */}
      <div>
        <h1 className="text-2xl font-bold tracking-tight flex items-center gap-2">
          <Code2 className="h-6 w-6 text-pink-500" aria-hidden />
          GraphQL Client
        </h1>
        <p className="text-sm text-muted-foreground mt-1">
          Execute GraphQL queries and introspect schemas
        </p>
      </div>

      {/* Endpoint URL + action buttons */}
      <div className="rounded-lg border border-border bg-card p-3">
        <div className="flex gap-2">
          <input
            type="text"
            value={endpointUrl}
            onChange={(e) => setEndpointUrl(e.target.value)}
            placeholder="https://api.example.com/graphql"
            aria-label="GraphQL endpoint URL"
            className={cn(
              "flex-1 h-9 px-3 rounded-md border border-input bg-background text-sm",
              "placeholder:text-muted-foreground",
              "focus:outline-none focus:ring-1 focus:ring-ring"
            )}
          />
          <Button
            type="button"
            variant="outline"
            onClick={() => void handleIntrospect()}
            disabled={introspectLoading || !endpointUrl.trim()}
            className="gap-2 shrink-0"
            aria-label="Introspect GraphQL schema"
          >
            {introspectLoading ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <Search className="h-4 w-4" />
            )}
            Introspect
          </Button>
          <Button
            type="button"
            onClick={() => void handleSend()}
            disabled={sendLoading || !endpointUrl.trim()}
            className="gap-2 shrink-0"
            aria-label="Send GraphQL query"
          >
            {sendLoading ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <Send className="h-4 w-4" />
            )}
            {sendLoading ? "Sending..." : "Send"}
          </Button>
        </div>
      </div>

      {/* Headers */}
      <HeaderEditor headers={headers} onChange={setHeaders} />

      {/* Query editor */}
      <div className="rounded-lg border border-border bg-card">
        <div className="px-4 py-3 border-b border-border">
          <h2 className="text-sm font-semibold">Query</h2>
        </div>
        <div className="p-3">
          <textarea
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            aria-label="GraphQL query"
            spellCheck={false}
            rows={12}
            className={cn(
              "w-full px-3 py-2 rounded-md border border-input bg-background text-sm font-mono resize-y",
              "placeholder:text-muted-foreground",
              "focus:outline-none focus:ring-1 focus:ring-ring"
            )}
          />
        </div>
      </div>

      {/* Variables + Operation Name — collapsible */}
      <CollapsibleSection title="Variables &amp; Options" defaultOpen={false}>
        <div className="p-3 space-y-4">
          {/* Operation name */}
          <div className="flex items-center gap-2">
            <label
              htmlFor="gql-operation-name"
              className="text-xs text-muted-foreground w-32 shrink-0"
            >
              Operation Name
            </label>
            <input
              id="gql-operation-name"
              type="text"
              value={operationName}
              onChange={(e) => setOperationName(e.target.value)}
              placeholder="Optional operation name"
              aria-label="GraphQL operation name"
              className={inputClass("flex-1")}
            />
          </div>

          {/* Variables */}
          <div className="space-y-1">
            <label
              htmlFor="gql-variables"
              className="text-xs text-muted-foreground"
            >
              Variables (JSON)
            </label>
            <textarea
              id="gql-variables"
              value={variables}
              onChange={(e) => setVariables(e.target.value)}
              placeholder={'{\n  "id": "123"\n}'}
              aria-label="GraphQL variables (JSON)"
              rows={6}
              spellCheck={false}
              className={cn(
                "w-full px-3 py-2 rounded-md border border-input bg-background text-sm font-mono resize-y",
                "placeholder:text-muted-foreground",
                "focus:outline-none focus:ring-1 focus:ring-ring"
              )}
            />
            {variables.trim() && (() => {
              try {
                JSON.parse(variables);
                return (
                  <p className="text-xs text-green-600 dark:text-green-400">
                    Valid JSON
                  </p>
                );
              } catch {
                return (
                  <p className="text-xs text-destructive">Invalid JSON</p>
                );
              }
            })()}
          </div>
        </div>
      </CollapsibleSection>

      {/* Send error */}
      {sendError && (
        <div
          role="alert"
          className="rounded-lg border border-destructive/50 bg-destructive/10 px-4 py-3"
        >
          <p className="text-sm text-destructive font-medium">Request failed</p>
          <p className="text-sm text-destructive/80 mt-1">{sendError}</p>
        </div>
      )}

      {/* Response */}
      {response && <ResponseViewer response={response} />}

      {/* Introspection error */}
      {introspectError && (
        <div
          role="alert"
          className="rounded-lg border border-destructive/50 bg-destructive/10 px-4 py-3"
        >
          <p className="text-sm text-destructive font-medium">Introspection failed</p>
          <p className="text-sm text-destructive/80 mt-1">{introspectError}</p>
        </div>
      )}

      {/* Schema introspection result */}
      {schema !== null && (
        <div className="rounded-lg border border-border bg-card">
          <div className="flex items-center justify-between px-4 py-3 border-b border-border">
            <h2 className="text-sm font-semibold">Schema</h2>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => setSchema(null)}
              aria-label="Clear schema"
              className="h-7 gap-1 text-xs text-muted-foreground"
            >
              <X className="h-3 w-3" />
              Clear
            </Button>
          </div>
          <div className="p-4 overflow-auto max-h-[480px]">
            <SchemaNode label="schema" value={schema} depth={0} />
          </div>
        </div>
      )}
    </div>
  );
}
