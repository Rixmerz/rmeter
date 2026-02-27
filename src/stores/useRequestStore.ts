import { create } from "zustand";
import type {
  HttpMethod,
  SendRequestInput,
  SendRequestOutput,
  Auth,
  RequestBody,
  HistoryEntry,
} from "@/types/request";
import {
  sendRequest as sendRequestCommand,
  getRequestHistory,
  clearRequestHistory,
} from "@/lib/commands";
import { parseCurl } from "@/lib/parseCurl";

export interface HeaderEntry {
  key: string;
  value: string;
  id: string;
}

export interface FormDataEntry {
  key: string;
  value: string;
  id: string;
}

export interface QueryParamEntry {
  key: string;
  value: string;
  id: string;
  enabled: boolean;
}

export type BodyType = "none" | "json" | "form_data" | "raw" | "xml";

interface RequestState {
  method: HttpMethod;
  url: string;
  headers: HeaderEntry[];
  bodyType: BodyType;
  bodyText: string;
  formDataEntries: FormDataEntry[];
  auth: Auth | null;
  queryParams: QueryParamEntry[];
  response: SendRequestOutput | null;
  loading: boolean;
  error: string | null;
  history: HistoryEntry[];
  historyLoaded: boolean;
}

interface RequestActions {
  setMethod: (method: HttpMethod) => void;
  setUrl: (url: string) => void;
  addHeader: () => void;
  updateHeader: (id: string, key: string, value: string) => void;
  removeHeader: (id: string) => void;
  setBodyType: (bodyType: BodyType) => void;
  setBodyText: (text: string) => void;
  addFormDataEntry: () => void;
  updateFormDataEntry: (id: string, key: string, value: string) => void;
  removeFormDataEntry: (id: string) => void;
  setAuth: (auth: Auth | null) => void;
  addQueryParam: () => void;
  updateQueryParam: (id: string, key: string, value: string) => void;
  toggleQueryParam: (id: string) => void;
  removeQueryParam: (id: string) => void;
  syncUrlToParams: (url: string) => void;
  sendRequest: () => Promise<void>;
  clearResponse: () => void;
  loadHistory: () => Promise<void>;
  clearHistory: () => Promise<void>;
  loadFromHistory: (entry: HistoryEntry) => void;
  loadFromCurl: (curlCommand: string) => void;
}

function buildBodyFromState(
  bodyType: BodyType,
  bodyText: string,
  formDataEntries: FormDataEntry[]
): RequestBody | null {
  switch (bodyType) {
    case "none":
      return null;
    case "json":
      return bodyText.trim() ? { type: "json", json: bodyText.trim() } : null;
    case "form_data": {
      const entries = formDataEntries
        .filter((e) => e.key.trim() !== "")
        .map((e): [string, string] => [e.key.trim(), e.value.trim()]);
      return entries.length > 0 ? { type: "form_data", form_data: entries } : null;
    }
    case "raw":
      return bodyText.trim() ? { type: "raw", raw: bodyText.trim() } : null;
    case "xml":
      return bodyText.trim() ? { type: "xml", xml: bodyText.trim() } : null;
    default:
      return null;
  }
}

function buildUrlFromParams(baseUrl: string, params: QueryParamEntry[]): string {
  const enabledParams = params.filter((p) => p.enabled && p.key.trim() !== "");
  if (enabledParams.length === 0) return baseUrl;

  try {
    // Parse existing URL to strip existing query
    const urlObj = new URL(baseUrl);
    urlObj.search = "";
    const newParams = new URLSearchParams();
    for (const p of enabledParams) {
      newParams.append(p.key.trim(), p.value.trim());
    }
    urlObj.search = newParams.toString();
    return urlObj.toString();
  } catch {
    // URL is incomplete/invalid, just build query string portion
    const [base] = baseUrl.split("?");
    const newParams = new URLSearchParams();
    for (const p of enabledParams) {
      newParams.append(p.key.trim(), p.value.trim());
    }
    const qs = newParams.toString();
    return qs ? `${base}?${qs}` : base;
  }
}

function parseParamsFromUrl(url: string): QueryParamEntry[] {
  try {
    const urlObj = new URL(url);
    const entries: QueryParamEntry[] = [];
    urlObj.searchParams.forEach((value, key) => {
      entries.push({ id: crypto.randomUUID(), key, value, enabled: true });
    });
    return entries;
  } catch {
    return [];
  }
}

export const useRequestStore = create<RequestState & RequestActions>((set, get) => ({
  method: "GET",
  url: "",
  headers: [],
  bodyType: "none",
  bodyText: "",
  formDataEntries: [],
  auth: null,
  queryParams: [],
  response: null,
  loading: false,
  error: null,
  history: [],
  historyLoaded: false,

  setMethod: (method) => set({ method }),

  setUrl: (url) => set({ url }),

  addHeader: () =>
    set((state) => ({
      headers: [
        ...state.headers,
        { id: crypto.randomUUID(), key: "", value: "" },
      ],
    })),

  updateHeader: (id, key, value) =>
    set((state) => ({
      headers: state.headers.map((h) =>
        h.id === id ? { ...h, key, value } : h
      ),
    })),

  removeHeader: (id) =>
    set((state) => ({
      headers: state.headers.filter((h) => h.id !== id),
    })),

  setBodyType: (bodyType) => set({ bodyType }),
  setBodyText: (bodyText) => set({ bodyText }),

  addFormDataEntry: () =>
    set((state) => ({
      formDataEntries: [
        ...state.formDataEntries,
        { id: crypto.randomUUID(), key: "", value: "" },
      ],
    })),

  updateFormDataEntry: (id, key, value) =>
    set((state) => ({
      formDataEntries: state.formDataEntries.map((e) =>
        e.id === id ? { ...e, key, value } : e
      ),
    })),

  removeFormDataEntry: (id) =>
    set((state) => ({
      formDataEntries: state.formDataEntries.filter((e) => e.id !== id),
    })),

  setAuth: (auth) => set({ auth }),

  addQueryParam: () =>
    set((state) => {
      const newParams = [
        ...state.queryParams,
        { id: crypto.randomUUID(), key: "", value: "", enabled: true },
      ];
      return {
        queryParams: newParams,
        url: buildUrlFromParams(state.url, newParams),
      };
    }),

  updateQueryParam: (id, key, value) =>
    set((state) => {
      const newParams = state.queryParams.map((p) =>
        p.id === id ? { ...p, key, value } : p
      );
      return {
        queryParams: newParams,
        url: buildUrlFromParams(state.url, newParams),
      };
    }),

  toggleQueryParam: (id) =>
    set((state) => {
      const newParams = state.queryParams.map((p) =>
        p.id === id ? { ...p, enabled: !p.enabled } : p
      );
      return {
        queryParams: newParams,
        url: buildUrlFromParams(state.url, newParams),
      };
    }),

  removeQueryParam: (id) =>
    set((state) => {
      const newParams = state.queryParams.filter((p) => p.id !== id);
      return {
        queryParams: newParams,
        url: buildUrlFromParams(state.url, newParams),
      };
    }),

  syncUrlToParams: (url) =>
    set({
      url,
      queryParams: parseParamsFromUrl(url),
    }),

  sendRequest: async () => {
    const { method, url, headers, bodyType, bodyText, formDataEntries, auth } = get();

    if (!url.trim()) {
      set({ error: "URL is required" });
      return;
    }

    const input: SendRequestInput = {
      method,
      url: url.trim(),
      headers: Object.fromEntries(
        headers
          .filter((h) => h.key.trim() !== "")
          .map((h) => [h.key.trim(), h.value.trim()])
      ),
      body: buildBodyFromState(bodyType, bodyText, formDataEntries),
      auth: auth ?? null,
    };

    set({ loading: true, error: null, response: null });

    try {
      const output = await sendRequestCommand(input);
      set({ response: output, loading: false });
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : String(err),
        loading: false,
      });
    }
  },

  clearResponse: () => set({ response: null, error: null }),

  loadHistory: async () => {
    try {
      const history = await getRequestHistory();
      set({ history, historyLoaded: true });
    } catch {
      set({ history: [], historyLoaded: true });
    }
  },

  clearHistory: async () => {
    try {
      await clearRequestHistory();
      set({ history: [] });
    } catch {
      // Ignore error — history clear is best-effort
    }
  },

  loadFromHistory: (entry) => {
    const { input } = entry;

    // Reconstruct bodyType + bodyText + formDataEntries from RequestBody
    let bodyType: BodyType = "none";
    let bodyText = "";
    let formDataEntries: FormDataEntry[] = [];

    if (input.body) {
      switch (input.body.type) {
        case "json":
          bodyType = "json";
          bodyText = input.body.json;
          break;
        case "form_data":
          bodyType = "form_data";
          formDataEntries = input.body.form_data.map(([key, value]) => ({
            id: crypto.randomUUID(),
            key,
            value,
          }));
          break;
        case "raw":
          bodyType = "raw";
          bodyText = input.body.raw;
          break;
        case "xml":
          bodyType = "xml";
          bodyText = input.body.xml;
          break;
      }
    }

    // Reconstruct headers array
    const headers: HeaderEntry[] = Object.entries(input.headers).map(
      ([key, value]) => ({ id: crypto.randomUUID(), key, value })
    );

    set({
      method: input.method as HttpMethod,
      url: input.url,
      headers,
      bodyType,
      bodyText,
      formDataEntries,
      auth: input.auth ?? null,
      queryParams: parseParamsFromUrl(input.url),
      response: null,
      error: null,
    });
  },

  loadFromCurl: (curlCommand) => {
    const parsed = parseCurl(curlCommand);

    const headers: HeaderEntry[] = Object.entries(parsed.headers).map(
      ([key, value]) => ({ id: crypto.randomUUID(), key, value })
    );

    let bodyType: BodyType = parsed.bodyType;
    let bodyText = parsed.body ?? "";
    let formDataEntries: FormDataEntry[] = [];

    if (parsed.bodyType === "form_data" && parsed.formData) {
      formDataEntries = parsed.formData.map(([key, value]) => ({
        id: crypto.randomUUID(),
        key,
        value,
      }));
    }

    set({
      method: parsed.method as HttpMethod,
      url: parsed.url,
      headers,
      bodyType,
      bodyText,
      formDataEntries,
      auth: parsed.auth,
      queryParams: parseParamsFromUrl(parsed.url),
      response: null,
      error: null,
    });
  },
}));
