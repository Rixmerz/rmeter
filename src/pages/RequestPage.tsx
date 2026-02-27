import { useEffect, useState, useRef } from "react";
import {
  Send,
  Plus,
  X,
  Loader2,
  ChevronDown,
  ChevronRight,
  History,
  Trash2,
  Terminal,
  ClipboardPaste,
} from "lucide-react";
import { Button } from "@/components/ui/button";
import { useRequestStore } from "@/stores/useRequestStore";
import type { HttpMethod, Auth, HistoryEntry } from "@/types/request";
import type { BodyType } from "@/stores/useRequestStore";
import { cn } from "@/lib/cn";

const HTTP_METHODS: HttpMethod[] = [
  "GET",
  "POST",
  "PUT",
  "DELETE",
  "PATCH",
  "HEAD",
  "OPTIONS",
];

const METHOD_COLORS: Record<HttpMethod, string> = {
  GET: "text-green-600 dark:text-green-400",
  POST: "text-blue-600 dark:text-blue-400",
  PUT: "text-yellow-600 dark:text-yellow-400",
  DELETE: "text-red-600 dark:text-red-400",
  PATCH: "text-orange-600 dark:text-orange-400",
  HEAD: "text-purple-600 dark:text-purple-400",
  OPTIONS: "text-cyan-600 dark:text-cyan-400",
};

const BODY_TYPES: { value: BodyType; label: string }[] = [
  { value: "none", label: "None" },
  { value: "json", label: "JSON" },
  { value: "form_data", label: "Form Data" },
  { value: "raw", label: "Raw" },
  { value: "xml", label: "XML" },
];

// ----------------------------------------------------------------
// Sub-components
// ----------------------------------------------------------------

interface SectionProps {
  title: string;
  children: React.ReactNode;
  defaultOpen?: boolean;
  action?: React.ReactNode;
}

function CollapsibleSection({ title, children, defaultOpen = true, action }: SectionProps) {
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

function inputClass(extra?: string) {
  return cn(
    "flex-1 h-8 px-3 rounded-md border border-input bg-background text-sm",
    "placeholder:text-muted-foreground",
    "focus:outline-none focus:ring-1 focus:ring-ring",
    extra
  );
}

// ----------------------------------------------------------------
// Auth Section
// ----------------------------------------------------------------

interface AuthSectionProps {
  auth: Auth | null;
  setAuth: (auth: Auth | null) => void;
}

function AuthSection({ auth, setAuth }: AuthSectionProps) {
  const authType = auth?.type ?? "none";

  const handleTypeChange = (value: string) => {
    if (value === "none") {
      setAuth(null);
    } else if (value === "bearer") {
      setAuth({ type: "bearer", token: auth?.token ?? "" });
    } else if (value === "basic") {
      setAuth({ type: "basic", username: auth?.username ?? "", password: auth?.password ?? "" });
    }
  };

  return (
    <CollapsibleSection title="Auth" defaultOpen={false}>
      <div className="p-3 space-y-3">
        <div className="flex items-center gap-2">
          <label className="text-xs text-muted-foreground w-24 shrink-0">Auth Type</label>
          <select
            value={authType}
            onChange={(e) => handleTypeChange(e.target.value)}
            aria-label="Authentication type"
            className={cn(
              "h-8 pl-3 pr-8 rounded-md border border-input bg-background text-sm",
              "focus:outline-none focus:ring-1 focus:ring-ring appearance-none cursor-pointer"
            )}
          >
            <option value="none">No Auth</option>
            <option value="bearer">Bearer Token</option>
            <option value="basic">Basic Auth</option>
          </select>
        </div>

        {auth?.type === "bearer" && (
          <div className="flex items-center gap-2">
            <label className="text-xs text-muted-foreground w-24 shrink-0">Token</label>
            <input
              type="text"
              value={auth.token ?? ""}
              onChange={(e) => setAuth({ ...auth, token: e.target.value })}
              placeholder="Bearer token value"
              aria-label="Bearer token"
              className={inputClass("flex-1")}
            />
          </div>
        )}

        {auth?.type === "basic" && (
          <>
            <div className="flex items-center gap-2">
              <label className="text-xs text-muted-foreground w-24 shrink-0">Username</label>
              <input
                type="text"
                value={auth.username ?? ""}
                onChange={(e) => setAuth({ ...auth, username: e.target.value })}
                placeholder="Username"
                aria-label="Basic auth username"
                className={inputClass("flex-1")}
              />
            </div>
            <div className="flex items-center gap-2">
              <label className="text-xs text-muted-foreground w-24 shrink-0">Password</label>
              <input
                type="password"
                value={auth.password ?? ""}
                onChange={(e) => setAuth({ ...auth, password: e.target.value })}
                placeholder="Password"
                aria-label="Basic auth password"
                className={inputClass("flex-1")}
              />
            </div>
          </>
        )}

        {authType === "none" && (
          <p className="text-xs text-muted-foreground">
            No authentication will be sent with this request.
          </p>
        )}
      </div>
    </CollapsibleSection>
  );
}

// ----------------------------------------------------------------
// Body Section
// ----------------------------------------------------------------

interface BodySectionProps {
  bodyType: BodyType;
  bodyText: string;
  formDataEntries: { key: string; value: string; id: string }[];
  setBodyType: (type: BodyType) => void;
  setBodyText: (text: string) => void;
  addFormDataEntry: () => void;
  updateFormDataEntry: (id: string, key: string, value: string) => void;
  removeFormDataEntry: (id: string) => void;
}

function BodySection({
  bodyType,
  bodyText,
  formDataEntries,
  setBodyType,
  setBodyText,
  addFormDataEntry,
  updateFormDataEntry,
  removeFormDataEntry,
}: BodySectionProps) {
  const isMonospace = bodyType === "json" || bodyType === "xml";
  const isTextarea = bodyType === "json" || bodyType === "raw" || bodyType === "xml";

  return (
    <div className="rounded-lg border border-border bg-card">
      <div className="flex items-center justify-between px-4 py-3 border-b border-border">
        <h2 className="text-sm font-semibold">Body</h2>
        <div className="flex gap-1">
          {BODY_TYPES.map((bt) => (
            <button
              key={bt.value}
              type="button"
              onClick={() => setBodyType(bt.value)}
              className={cn(
                "px-2.5 py-1 rounded text-xs font-medium transition-colors",
                bodyType === bt.value
                  ? "bg-primary text-primary-foreground"
                  : "text-muted-foreground hover:text-foreground hover:bg-accent"
              )}
              aria-pressed={bodyType === bt.value}
              aria-label={`Body type ${bt.label}`}
            >
              {bt.label}
            </button>
          ))}
        </div>
      </div>

      <div className="p-3">
        {bodyType === "none" && (
          <p className="text-sm text-muted-foreground text-center py-4">
            This request has no body.
          </p>
        )}

        {isTextarea && (
          <textarea
            value={bodyText}
            onChange={(e) => setBodyText(e.target.value)}
            placeholder={
              bodyType === "json"
                ? '{"key": "value"}'
                : bodyType === "xml"
                ? "<root><item>value</item></root>"
                : "Request body content"
            }
            aria-label="Request body"
            rows={8}
            className={cn(
              "w-full px-3 py-2 rounded-md border border-input bg-background text-sm resize-y",
              "placeholder:text-muted-foreground",
              "focus:outline-none focus:ring-1 focus:ring-ring",
              isMonospace && "font-mono"
            )}
          />
        )}

        {bodyType === "form_data" && (
          <div className="space-y-2">
            {formDataEntries.length === 0 ? (
              <p className="text-sm text-muted-foreground text-center py-4">
                No form fields. Click "Add Field" to add one.
              </p>
            ) : (
              formDataEntries.map((entry) => (
                <div key={entry.id} className="flex gap-2 items-center">
                  <input
                    type="text"
                    value={entry.key}
                    onChange={(e) =>
                      updateFormDataEntry(entry.id, e.target.value, entry.value)
                    }
                    placeholder="Field name"
                    aria-label="Form field name"
                    className={inputClass()}
                  />
                  <input
                    type="text"
                    value={entry.value}
                    onChange={(e) =>
                      updateFormDataEntry(entry.id, entry.key, e.target.value)
                    }
                    placeholder="Field value"
                    aria-label="Form field value"
                    className={inputClass()}
                  />
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    onClick={() => removeFormDataEntry(entry.id)}
                    aria-label="Remove form field"
                    className="h-8 w-8 text-muted-foreground hover:text-destructive"
                  >
                    <X className="h-3.5 w-3.5" />
                  </Button>
                </div>
              ))
            )}
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={addFormDataEntry}
              className="h-7 gap-1 text-xs mt-1"
            >
              <Plus className="h-3 w-3" />
              Add Field
            </Button>
          </div>
        )}
      </div>
    </div>
  );
}

// ----------------------------------------------------------------
// Query Params Section
// ----------------------------------------------------------------

interface QueryParamsSectionProps {
  queryParams: { key: string; value: string; id: string; enabled: boolean }[];
  addQueryParam: () => void;
  updateQueryParam: (id: string, key: string, value: string) => void;
  toggleQueryParam: (id: string) => void;
  removeQueryParam: (id: string) => void;
}

function QueryParamsSection({
  queryParams,
  addQueryParam,
  updateQueryParam,
  toggleQueryParam,
  removeQueryParam,
}: QueryParamsSectionProps) {
  return (
    <CollapsibleSection
      title={`Query Params ${queryParams.length > 0 ? `(${queryParams.length})` : ""}`}
      defaultOpen={false}
      action={
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={addQueryParam}
          className="h-7 gap-1 text-xs"
        >
          <Plus className="h-3 w-3" />
          Add Param
        </Button>
      }
    >
      <div className="p-3 space-y-2">
        {queryParams.length === 0 ? (
          <p className="text-sm text-muted-foreground text-center py-4">
            No query parameters. Click "Add Param" to add one.
          </p>
        ) : (
          queryParams.map((param) => (
            <div key={param.id} className="flex gap-2 items-center">
              <input
                type="checkbox"
                checked={param.enabled}
                onChange={() => toggleQueryParam(param.id)}
                aria-label="Enable parameter"
                className="h-4 w-4 rounded border border-input accent-primary cursor-pointer shrink-0"
              />
              <input
                type="text"
                value={param.key}
                onChange={(e) =>
                  updateQueryParam(param.id, e.target.value, param.value)
                }
                placeholder="Key"
                aria-label="Query parameter key"
                className={cn(inputClass(), !param.enabled && "opacity-50")}
              />
              <input
                type="text"
                value={param.value}
                onChange={(e) =>
                  updateQueryParam(param.id, param.key, e.target.value)
                }
                placeholder="Value"
                aria-label="Query parameter value"
                className={cn(inputClass(), !param.enabled && "opacity-50")}
              />
              <Button
                type="button"
                variant="ghost"
                size="icon"
                onClick={() => removeQueryParam(param.id)}
                aria-label="Remove query parameter"
                className="h-8 w-8 text-muted-foreground hover:text-destructive"
              >
                <X className="h-3.5 w-3.5" />
              </Button>
            </div>
          ))
        )}
      </div>
    </CollapsibleSection>
  );
}

// ----------------------------------------------------------------
// History Panel
// ----------------------------------------------------------------

interface HistoryPanelProps {
  history: HistoryEntry[];
  onLoad: (entry: HistoryEntry) => void;
  onClear: () => void;
  onClose: () => void;
}

function getStatusColor(status: number): string {
  if (status >= 200 && status < 300) return "text-green-600 dark:text-green-400";
  if (status >= 300 && status < 400) return "text-yellow-600 dark:text-yellow-400";
  if (status >= 400 && status < 500) return "text-red-600 dark:text-red-400";
  if (status >= 500) return "text-red-700 dark:text-red-300";
  return "text-muted-foreground";
}

function HistoryPanel({ history, onLoad, onClear, onClose }: HistoryPanelProps) {
  return (
    <div className="rounded-lg border border-border bg-card flex flex-col h-full max-h-[480px]">
      <div className="flex items-center justify-between px-4 py-3 border-b border-border shrink-0">
        <div className="flex items-center gap-2">
          <History className="h-4 w-4 text-muted-foreground" />
          <h2 className="text-sm font-semibold">Request History</h2>
          {history.length > 0 && (
            <span className="text-xs text-muted-foreground">({history.length})</span>
          )}
        </div>
        <div className="flex items-center gap-1">
          {history.length > 0 && (
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={onClear}
              className="h-7 gap-1 text-xs text-muted-foreground hover:text-destructive"
              aria-label="Clear history"
            >
              <Trash2 className="h-3 w-3" />
              Clear
            </Button>
          )}
          <Button
            type="button"
            variant="ghost"
            size="icon"
            onClick={onClose}
            aria-label="Close history panel"
            className="h-7 w-7 text-muted-foreground"
          >
            <X className="h-3.5 w-3.5" />
          </Button>
        </div>
      </div>

      <div className="overflow-y-auto flex-1">
        {history.length === 0 ? (
          <p className="text-sm text-muted-foreground text-center py-8 px-4">
            No request history yet. Send a request to see it here.
          </p>
        ) : (
          <ul role="list" className="divide-y divide-border">
            {history.map((entry) => (
              <li key={entry.id}>
                <button
                  type="button"
                  onClick={() => onLoad(entry)}
                  className="w-full px-4 py-3 text-left hover:bg-accent/50 transition-colors group"
                  aria-label={`Load ${entry.input.method} ${entry.input.url}`}
                >
                  <div className="flex items-center gap-2 min-w-0">
                    <span
                      className={cn(
                        "text-xs font-bold shrink-0 font-mono",
                        METHOD_COLORS[entry.input.method as HttpMethod] ??
                          "text-muted-foreground"
                      )}
                    >
                      {entry.input.method}
                    </span>
                    <span className="text-xs text-foreground truncate flex-1 font-mono">
                      {entry.input.url}
                    </span>
                    <span
                      className={cn(
                        "text-xs font-semibold shrink-0",
                        getStatusColor(entry.output.status)
                      )}
                      aria-label={`Status ${entry.output.status}`}
                    >
                      {entry.output.status}
                    </span>
                  </div>
                  <div className="flex items-center gap-2 mt-0.5">
                    <span className="text-xs text-muted-foreground">
                      {new Date(entry.timestamp).toLocaleTimeString()}
                    </span>
                    <span className="text-xs text-muted-foreground">
                      {entry.output.elapsed_ms}ms
                    </span>
                  </div>
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}

// ----------------------------------------------------------------
// Import cURL Dialog
// ----------------------------------------------------------------

interface ImportCurlDialogProps {
  open: boolean;
  onClose: () => void;
  onImport: (curlCommand: string) => void;
}

function ImportCurlDialog({ open, onClose, onImport }: ImportCurlDialogProps) {
  const [curlText, setCurlText] = useState("");
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (open) {
      setCurlText("");
      setTimeout(() => textareaRef.current?.focus(), 50);
    }
  }, [open]);

  if (!open) return null;

  function handleImport() {
    const trimmed = curlText.trim();
    if (!trimmed) return;
    onImport(trimmed);
    onClose();
  }

  async function handlePaste() {
    try {
      const text = await navigator.clipboard.readText();
      setCurlText(text);
    } catch {
      // Clipboard API may fail in some contexts
    }
  }

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-black/50"
      onClick={(e) => { if (e.target === e.currentTarget) onClose(); }}
      role="dialog"
      aria-modal="true"
      aria-label="Import cURL command"
    >
      <div className="bg-card border border-border rounded-lg shadow-lg w-full max-w-2xl mx-4">
        <div className="flex items-center justify-between px-4 py-3 border-b border-border">
          <div className="flex items-center gap-2">
            <Terminal className="h-4 w-4 text-muted-foreground" />
            <h2 className="text-sm font-semibold">Import cURL</h2>
          </div>
          <button
            onClick={onClose}
            aria-label="Close"
            className="p-1 rounded hover:bg-muted text-muted-foreground hover:text-foreground"
          >
            <X className="h-4 w-4" />
          </button>
        </div>

        <div className="p-4 space-y-3">
          <p className="text-xs text-muted-foreground">
            Paste a cURL command and it will be automatically parsed into method, URL, headers, body, and auth.
          </p>

          <div className="relative">
            <textarea
              ref={textareaRef}
              value={curlText}
              onChange={(e) => setCurlText(e.target.value)}
              placeholder={"curl -X POST https://api.example.com/users \\\n  -H 'Content-Type: application/json' \\\n  -H 'Authorization: Bearer token123' \\\n  -d '{\"name\": \"John\", \"email\": \"john@example.com\"}'"}
              rows={8}
              className={cn(
                "w-full px-3 py-2 rounded-md border border-input bg-background text-sm font-mono resize-y",
                "placeholder:text-muted-foreground/50",
                "focus:outline-none focus:ring-1 focus:ring-ring"
              )}
            />
            <button
              onClick={() => void handlePaste()}
              aria-label="Paste from clipboard"
              className={cn(
                "absolute top-2 right-2 flex items-center gap-1 px-2 py-1 rounded text-xs",
                "bg-muted hover:bg-muted/80 text-muted-foreground hover:text-foreground transition-colors"
              )}
            >
              <ClipboardPaste className="h-3 w-3" />
              Paste
            </button>
          </div>

          <div className="flex gap-2 justify-end">
            <Button variant="outline" size="sm" onClick={onClose}>
              Cancel
            </Button>
            <Button
              size="sm"
              onClick={handleImport}
              disabled={!curlText.trim()}
              className="gap-1.5"
            >
              <Terminal className="h-3.5 w-3.5" />
              Import
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}

// ----------------------------------------------------------------
// Main Page
// ----------------------------------------------------------------

export function RequestPage() {
  const {
    method,
    url,
    headers,
    bodyType,
    bodyText,
    formDataEntries,
    auth,
    queryParams,
    response,
    loading,
    error,
    history,
    historyLoaded,
    setMethod,
    addHeader,
    updateHeader,
    removeHeader,
    setBodyType,
    setBodyText,
    addFormDataEntry,
    updateFormDataEntry,
    removeFormDataEntry,
    setAuth,
    addQueryParam,
    updateQueryParam,
    toggleQueryParam,
    removeQueryParam,
    syncUrlToParams,
    sendRequest,
    loadHistory,
    clearHistory,
    loadFromHistory,
    loadFromCurl,
  } = useRequestStore();

  const [showHistory, setShowHistory] = useState(false);
  const [showCurlImport, setShowCurlImport] = useState(false);

  useEffect(() => {
    if (showHistory && !historyLoaded) {
      void loadHistory();
    }
  }, [showHistory, historyLoaded, loadHistory]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    void sendRequest();
  };

  const handleUrlChange = (value: string) => {
    syncUrlToParams(value);
  };

  const handleToggleHistory = () => {
    setShowHistory((v) => !v);
  };

  const handleLoadFromHistory = (entry: HistoryEntry) => {
    loadFromHistory(entry);
    setShowHistory(false);
  };

  return (
    <div className="max-w-5xl mx-auto space-y-6 p-6">
      {/* Header */}
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">HTTP Request Builder</h1>
          <p className="text-sm text-muted-foreground mt-1">
            Send HTTP requests and inspect responses
          </p>
        </div>
        <div className="flex items-center gap-2 shrink-0">
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={() => setShowCurlImport(true)}
            className="gap-2"
            aria-label="Import cURL command"
          >
            <Terminal className="h-4 w-4" />
            Import cURL
          </Button>
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={handleToggleHistory}
            className="gap-2"
            aria-pressed={showHistory}
            aria-label="Toggle request history"
          >
            <History className="h-4 w-4" />
            History
            {history.length > 0 && (
              <span className="ml-1 text-xs bg-muted rounded-full px-1.5 py-0.5">
                {history.length}
              </span>
            )}
          </Button>
        </div>
      </div>

      {/* History Panel */}
      {showHistory && (
        <HistoryPanel
          history={history}
          onLoad={handleLoadFromHistory}
          onClear={() => void clearHistory()}
          onClose={() => setShowHistory(false)}
        />
      )}

      <form onSubmit={handleSubmit} className="space-y-4">
        {/* Method + URL row */}
        <div className="flex gap-2">
          <div className="relative">
            <select
              value={method}
              onChange={(e) => setMethod(e.target.value as HttpMethod)}
              aria-label="HTTP method"
              className={cn(
                "h-9 pl-3 pr-8 rounded-md border border-input bg-background text-sm font-semibold",
                "focus:outline-none focus:ring-1 focus:ring-ring",
                "appearance-none cursor-pointer",
                METHOD_COLORS[method]
              )}
            >
              {HTTP_METHODS.map((m) => (
                <option key={m} value={m} className="text-foreground">
                  {m}
                </option>
              ))}
            </select>
          </div>

          <input
            type="text"
            value={url}
            onChange={(e) => handleUrlChange(e.target.value)}
            placeholder="https://api.example.com/endpoint"
            aria-label="Request URL"
            required
            className={cn(
              "flex-1 h-9 px-3 rounded-md border border-input bg-background text-sm",
              "placeholder:text-muted-foreground",
              "focus:outline-none focus:ring-1 focus:ring-ring"
            )}
          />

          <Button
            type="submit"
            disabled={loading || !url.trim()}
            className="gap-2"
          >
            {loading ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <Send className="h-4 w-4" />
            )}
            {loading ? "Sending..." : "Send"}
          </Button>
        </div>

        {/* Query Params */}
        <QueryParamsSection
          queryParams={queryParams}
          addQueryParam={addQueryParam}
          updateQueryParam={updateQueryParam}
          toggleQueryParam={toggleQueryParam}
          removeQueryParam={removeQueryParam}
        />

        {/* Headers section */}
        <CollapsibleSection
          title="Headers"
          action={
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={addHeader}
              className="h-7 gap-1 text-xs"
            >
              <Plus className="h-3 w-3" />
              Add Header
            </Button>
          }
        >
          <div className="p-3 space-y-2">
            {headers.length === 0 ? (
              <p className="text-sm text-muted-foreground text-center py-4">
                No headers. Click "Add Header" to add one.
              </p>
            ) : (
              headers.map((header) => (
                <div key={header.id} className="flex gap-2 items-center">
                  <input
                    type="text"
                    value={header.key}
                    onChange={(e) =>
                      updateHeader(header.id, e.target.value, header.value)
                    }
                    placeholder="Header name"
                    aria-label="Header name"
                    className={inputClass()}
                  />
                  <input
                    type="text"
                    value={header.value}
                    onChange={(e) =>
                      updateHeader(header.id, header.key, e.target.value)
                    }
                    placeholder="Header value"
                    aria-label="Header value"
                    className={inputClass()}
                  />
                  <Button
                    type="button"
                    variant="ghost"
                    size="icon"
                    onClick={() => removeHeader(header.id)}
                    aria-label="Remove header"
                    className="h-8 w-8 text-muted-foreground hover:text-destructive"
                  >
                    <X className="h-3.5 w-3.5" />
                  </Button>
                </div>
              ))
            )}
          </div>
        </CollapsibleSection>

        {/* Auth section */}
        <AuthSection auth={auth} setAuth={setAuth} />

        {/* Body section */}
        <BodySection
          bodyType={bodyType}
          bodyText={bodyText}
          formDataEntries={formDataEntries}
          setBodyType={setBodyType}
          setBodyText={setBodyText}
          addFormDataEntry={addFormDataEntry}
          updateFormDataEntry={updateFormDataEntry}
          removeFormDataEntry={removeFormDataEntry}
        />
      </form>

      {/* Error state */}
      {error && (
        <div
          role="alert"
          className="rounded-lg border border-destructive/50 bg-destructive/10 px-4 py-3"
        >
          <p className="text-sm text-destructive font-medium">Request failed</p>
          <p className="text-sm text-destructive/80 mt-1">{error}</p>
        </div>
      )}

      {/* Response section */}
      {response && (
        <div className="rounded-lg border border-border bg-card">
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

          {/* Response headers */}
          <div className="px-4 py-3 border-b border-border">
            <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-2">
              Response Headers
            </h3>
            <div className="space-y-1">
              {Object.entries(response.headers).map(([key, value]) => (
                <div key={key} className="flex gap-2 text-xs font-mono">
                  <span className="text-muted-foreground min-w-[160px] truncate">{key}</span>
                  <span className="text-foreground break-all">{value}</span>
                </div>
              ))}
            </div>
          </div>

          {/* Response body */}
          <div className="p-4">
            <h3 className="text-xs font-semibold uppercase tracking-wider text-muted-foreground mb-2">
              Response Body
            </h3>
            <pre className="text-sm font-mono bg-muted/50 rounded-md p-3 overflow-auto max-h-96 whitespace-pre-wrap break-all">
              {(() => {
                try {
                  return JSON.stringify(JSON.parse(response.body), null, 2);
                } catch {
                  return response.body;
                }
              })()}
            </pre>
          </div>
        </div>
      )}

      {/* Import cURL dialog */}
      <ImportCurlDialog
        open={showCurlImport}
        onClose={() => setShowCurlImport(false)}
        onImport={loadFromCurl}
      />
    </div>
  );
}
