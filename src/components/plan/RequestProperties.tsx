import { useState, useEffect } from "react";
import { Plus, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/cn";
import type { HttpRequest, RequestBody } from "@/types/plan";
import { usePlanStore } from "@/stores/usePlanStore";
import { AssertionEditor } from "./AssertionEditor";
import { ExtractorEditor } from "./ExtractorEditor";
import { useEngineStore } from "@/stores/useEngineStore";

const HTTP_METHODS = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"] as const;
type HttpMethod = (typeof HTTP_METHODS)[number];

const METHOD_COLORS: Record<HttpMethod, string> = {
  GET: "text-green-600 dark:text-green-400",
  POST: "text-blue-600 dark:text-blue-400",
  PUT: "text-yellow-600 dark:text-yellow-400",
  DELETE: "text-red-600 dark:text-red-400",
  PATCH: "text-orange-600 dark:text-orange-400",
  HEAD: "text-purple-600 dark:text-purple-400",
  OPTIONS: "text-cyan-600 dark:text-cyan-400",
};

const BODY_TYPES = ["none", "json", "raw", "xml", "form_data"] as const;
type BodyType = (typeof BODY_TYPES)[number];

const BODY_TYPE_LABELS: Record<BodyType, string> = {
  none: "None",
  json: "JSON",
  raw: "Raw",
  xml: "XML",
  form_data: "Form Data",
};

const inputClass = cn(
  "w-full text-sm px-2 py-1.5 rounded border border-input bg-background",
  "focus:outline-none focus:ring-1 focus:ring-ring"
);

const textareaClass = cn(
  "w-full text-sm px-2 py-1.5 rounded border border-input bg-background font-mono",
  "focus:outline-none focus:ring-1 focus:ring-ring resize-y min-h-[80px]"
);

interface FieldProps {
  label: string;
  htmlFor: string;
  children: React.ReactNode;
}

function Field({ label, htmlFor, children }: FieldProps) {
  return (
    <div className="space-y-1">
      <label htmlFor={htmlFor} className="text-xs font-medium text-muted-foreground block">
        {label}
      </label>
      {children}
    </div>
  );
}

interface HeaderEntry {
  key: string;
  value: string;
  localId: string;
}

interface FormDataEntry {
  key: string;
  value: string;
  localId: string;
}

function headersFromRecord(record: Record<string, string>): HeaderEntry[] {
  return Object.entries(record).map(([key, value]) => ({
    key,
    value,
    localId: crypto.randomUUID(),
  }));
}

function headersToRecord(entries: HeaderEntry[]): Record<string, string> {
  const result: Record<string, string> = {};
  for (const entry of entries) {
    const k = entry.key.trim();
    if (k) result[k] = entry.value;
  }
  return result;
}

// --- Body helpers ---

function getBodyType(body: RequestBody | null): BodyType {
  if (!body) return "none";
  return body.type;
}

function getBodyContent(body: RequestBody | null): string {
  if (!body) return "";
  switch (body.type) {
    case "json": return body.json;
    case "raw": return body.raw;
    case "xml": return body.xml;
    case "form_data": return "";
  }
}

function getFormDataEntries(body: RequestBody | null): FormDataEntry[] {
  if (!body || body.type !== "form_data") return [];
  return body.form_data.map(([key, value]) => ({
    key,
    value,
    localId: crypto.randomUUID(),
  }));
}

function buildBody(type: BodyType, content: string, formEntries: FormDataEntry[]): RequestBody | null {
  switch (type) {
    case "none": return null;
    case "json": return { type: "json", json: content };
    case "raw": return { type: "raw", raw: content };
    case "xml": return { type: "xml", xml: content };
    case "form_data":
      return {
        type: "form_data",
        form_data: formEntries
          .filter((e) => e.key.trim())
          .map((e) => [e.key, e.value]),
      };
  }
}

interface RequestPropertiesProps {
  request: HttpRequest;
  groupId: string;
}

export function RequestProperties({ request, groupId }: RequestPropertiesProps) {
  const { updateRequest, activePlan, loadPlans } = usePlanStore();
  const recentResults = useEngineStore((s) => s.recentResults);

  const latestResultForRequest = [...recentResults]
    .reverse()
    .find((r) => r.request_name === request.name);
  const assertionResults = latestResultForRequest?.assertion_results ?? [];
  const extractionResults = latestResultForRequest?.extraction_results ?? [];

  const [name, setName] = useState(request.name);
  const [method, setMethod] = useState<string>(request.method);
  const [url, setUrl] = useState(request.url);
  const [headers, setHeaders] = useState<HeaderEntry[]>(() =>
    headersFromRecord(request.headers)
  );
  const [bodyType, setBodyType] = useState<BodyType>(() => getBodyType(request.body));
  const [bodyContent, setBodyContent] = useState(() => getBodyContent(request.body));
  const [formEntries, setFormEntries] = useState<FormDataEntry[]>(() =>
    getFormDataEntries(request.body)
  );
  const [dirty, setDirty] = useState(false);

  // Sync when selected request changes
  useEffect(() => {
    setName(request.name);
    setMethod(request.method);
    setUrl(request.url);
    setHeaders(headersFromRecord(request.headers));
    setBodyType(getBodyType(request.body));
    setBodyContent(getBodyContent(request.body));
    setFormEntries(getFormDataEntries(request.body));
    setDirty(false);
  }, [request.id, request.name, request.method, request.url, request.headers, request.body]);

  function markDirty() {
    setDirty(true);
  }

  async function handleSave() {
    await updateRequest(groupId, request.id, {
      name: name.trim() || request.name,
      method,
      url: url.trim(),
      headers: headersToRecord(headers),
      body: buildBody(bodyType, bodyContent, formEntries),
    });
    setDirty(false);
  }

  function handleDiscard() {
    setName(request.name);
    setMethod(request.method);
    setUrl(request.url);
    setHeaders(headersFromRecord(request.headers));
    setBodyType(getBodyType(request.body));
    setBodyContent(getBodyContent(request.body));
    setFormEntries(getFormDataEntries(request.body));
    setDirty(false);
  }

  function addHeader() {
    setHeaders((prev) => [...prev, { key: "", value: "", localId: crypto.randomUUID() }]);
    markDirty();
  }

  function updateHeader(localId: string, field: "key" | "value", val: string) {
    setHeaders((prev) =>
      prev.map((h) => (h.localId === localId ? { ...h, [field]: val } : h))
    );
    markDirty();
  }

  function removeHeader(localId: string) {
    setHeaders((prev) => prev.filter((h) => h.localId !== localId));
    markDirty();
  }

  // Form data helpers
  function addFormEntry() {
    setFormEntries((prev) => [...prev, { key: "", value: "", localId: crypto.randomUUID() }]);
    markDirty();
  }

  function updateFormEntry(localId: string, field: "key" | "value", val: string) {
    setFormEntries((prev) =>
      prev.map((e) => (e.localId === localId ? { ...e, [field]: val } : e))
    );
    markDirty();
  }

  function removeFormEntry(localId: string) {
    setFormEntries((prev) => prev.filter((e) => e.localId !== localId));
    markDirty();
  }

  return (
    <div className="space-y-4">
      <h3 className="text-sm font-semibold">HTTP Request</h3>

      <Field label="Name" htmlFor="req-name">
        <input
          id="req-name"
          type="text"
          value={name}
          onChange={(e) => { setName(e.target.value); markDirty(); }}
          className={inputClass}
        />
      </Field>

      <div className="space-y-1">
        <span className="text-xs font-medium text-muted-foreground block">Method &amp; URL</span>
        <div className="flex gap-2">
          <select
            aria-label="HTTP method"
            value={method}
            onChange={(e) => { setMethod(e.target.value); markDirty(); }}
            className={cn(
              "text-sm px-2 py-1.5 rounded border border-input bg-background font-mono font-bold",
              "focus:outline-none focus:ring-1 focus:ring-ring",
              METHOD_COLORS[method as HttpMethod] ?? "text-foreground"
            )}
          >
            {HTTP_METHODS.map((m) => (
              <option key={m} value={m}>
                {m}
              </option>
            ))}
          </select>
          <input
            id="req-url"
            type="url"
            value={url}
            onChange={(e) => { setUrl(e.target.value); markDirty(); }}
            placeholder="https://example.com/api/${version}/users"
            aria-label="Request URL"
            className={cn(inputClass, "flex-1")}
          />
        </div>
      </div>

      {/* Headers */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <span className="text-xs font-medium text-muted-foreground">Headers</span>
          <button
            onClick={addHeader}
            aria-label="Add header"
            className={cn(
              "flex items-center gap-1 text-xs text-muted-foreground",
              "hover:text-foreground",
              "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring rounded"
            )}
          >
            <Plus className="h-3 w-3" />
            Add
          </button>
        </div>
        {headers.length === 0 && (
          <p className="text-xs text-muted-foreground italic">No headers</p>
        )}
        {headers.map((header) => (
          <div key={header.localId} className="flex gap-1.5 items-center">
            <input
              type="text"
              value={header.key}
              onChange={(e) => updateHeader(header.localId, "key", e.target.value)}
              placeholder="Header name"
              aria-label="Header name"
              className={cn(inputClass, "flex-1")}
            />
            <input
              type="text"
              value={header.value}
              onChange={(e) => updateHeader(header.localId, "value", e.target.value)}
              placeholder="Value"
              aria-label="Header value"
              className={cn(inputClass, "flex-1")}
            />
            <button
              onClick={() => removeHeader(header.localId)}
              aria-label="Remove header"
              className={cn(
                "p-1 rounded hover:bg-destructive/10 hover:text-destructive shrink-0",
                "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
              )}
            >
              <Trash2 className="h-3.5 w-3.5" />
            </button>
          </div>
        ))}
      </div>

      {/* Body */}
      <div className="space-y-2">
        <div className="flex items-center justify-between">
          <span className="text-xs font-medium text-muted-foreground">Body</span>
          <select
            aria-label="Body type"
            value={bodyType}
            onChange={(e) => {
              const newType = e.target.value as BodyType;
              setBodyType(newType);
              if (newType === "form_data" && formEntries.length === 0) {
                setFormEntries([{ key: "", value: "", localId: crypto.randomUUID() }]);
              }
              markDirty();
            }}
            className={cn(
              "text-xs px-2 py-1 rounded border border-input bg-background",
              "focus:outline-none focus:ring-1 focus:ring-ring"
            )}
          >
            {BODY_TYPES.map((t) => (
              <option key={t} value={t}>{BODY_TYPE_LABELS[t]}</option>
            ))}
          </select>
        </div>

        {bodyType === "none" && (
          <p className="text-xs text-muted-foreground italic">No body. Select a type above to add one.</p>
        )}

        {(bodyType === "json" || bodyType === "raw" || bodyType === "xml") && (
          <div className="space-y-1">
            <textarea
              id="req-body"
              value={bodyContent}
              onChange={(e) => { setBodyContent(e.target.value); markDirty(); }}
              placeholder={
                bodyType === "json"
                  ? '{"key": "${varName}", "count": 10}'
                  : bodyType === "xml"
                  ? "<request><user>${userName}</user></request>"
                  : "Raw body content with ${variables}"
              }
              aria-label="Request body"
              className={textareaClass}
              rows={6}
            />
            <p className="text-[10px] text-muted-foreground">
              Use <code className="font-mono bg-muted px-1 rounded">{"${varName}"}</code> for dynamic variables
            </p>
          </div>
        )}

        {bodyType === "form_data" && (
          <div className="space-y-2">
            {formEntries.map((entry) => (
              <div key={entry.localId} className="flex gap-1.5 items-center">
                <input
                  type="text"
                  value={entry.key}
                  onChange={(e) => updateFormEntry(entry.localId, "key", e.target.value)}
                  placeholder="Field name"
                  aria-label="Form field name"
                  className={cn(inputClass, "flex-1")}
                />
                <input
                  type="text"
                  value={entry.value}
                  onChange={(e) => updateFormEntry(entry.localId, "value", e.target.value)}
                  placeholder="${varName} or value"
                  aria-label="Form field value"
                  className={cn(inputClass, "flex-1")}
                />
                <button
                  onClick={() => removeFormEntry(entry.localId)}
                  aria-label="Remove form field"
                  className={cn(
                    "p-1 rounded hover:bg-destructive/10 hover:text-destructive shrink-0",
                    "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring"
                  )}
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </button>
              </div>
            ))}
            <button
              onClick={addFormEntry}
              aria-label="Add form field"
              className={cn(
                "flex items-center gap-1 text-xs text-muted-foreground",
                "hover:text-foreground",
                "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring rounded"
              )}
            >
              <Plus className="h-3 w-3" />
              Add field
            </button>
            <p className="text-[10px] text-muted-foreground">
              Use <code className="font-mono bg-muted px-1 rounded">{"${varName}"}</code> in keys or values
            </p>
          </div>
        )}
      </div>

      <Field label="Status" htmlFor="req-enabled">
        <div className="flex items-center gap-2">
          <input
            id="req-enabled"
            type="checkbox"
            checked={request.enabled}
            readOnly
            className="h-4 w-4 rounded border border-input"
            aria-label="Enabled"
          />
          <span className="text-sm">{request.enabled ? "Enabled" : "Disabled"}</span>
          <span className="text-xs text-muted-foreground">(use tree context menu to toggle)</span>
        </div>
      </Field>

      {activePlan && (
        <AssertionEditor
          planId={activePlan.id}
          groupId={groupId}
          requestId={request.id}
          assertions={request.assertions}
          assertionResults={assertionResults}
          onAssertionsChange={() => void loadPlans()}
        />
      )}

      {activePlan && (
        <ExtractorEditor
          planId={activePlan.id}
          groupId={groupId}
          requestId={request.id}
          extractors={request.extractors}
          extractionResults={extractionResults}
          onExtractorsChange={() => void loadPlans()}
        />
      )}

      {dirty && (
        <div className="flex gap-2 pt-2 border-t border-border">
          <Button size="sm" onClick={() => void handleSave()} className="flex-1">
            Apply Changes
          </Button>
          <Button size="sm" variant="outline" onClick={handleDiscard} className="flex-1">
            Discard
          </Button>
        </div>
      )}
    </div>
  );
}
