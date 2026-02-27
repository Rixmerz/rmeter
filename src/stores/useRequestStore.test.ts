import { describe, it, expect, beforeEach, vi } from "vitest";
import { useRequestStore } from "./useRequestStore";
import type { HistoryEntry } from "@/types/request";

// Mock the commands module
vi.mock("@/lib/commands", () => ({
  sendRequest: vi.fn(),
  getRequestHistory: vi.fn(() => Promise.resolve([])),
  clearRequestHistory: vi.fn(() => Promise.resolve()),
}));

// Mock crypto.randomUUID for deterministic tests
let uuidCounter = 0;
vi.stubGlobal("crypto", {
  randomUUID: () => `mock-uuid-${++uuidCounter}`,
});

function resetStore() {
  useRequestStore.setState({
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
  });
}

describe("useRequestStore", () => {
  beforeEach(() => {
    uuidCounter = 0;
    resetStore();
  });

  // -----------------------------------------------------------------------
  // Initial State
  // -----------------------------------------------------------------------

  describe("initial state", () => {
    it("starts with GET method", () => {
      expect(useRequestStore.getState().method).toBe("GET");
    });

    it("starts with empty url", () => {
      expect(useRequestStore.getState().url).toBe("");
    });

    it("starts with no headers", () => {
      expect(useRequestStore.getState().headers).toEqual([]);
    });

    it("starts with body type none", () => {
      expect(useRequestStore.getState().bodyType).toBe("none");
    });

    it("starts with no response", () => {
      expect(useRequestStore.getState().response).toBeNull();
    });

    it("starts not loading", () => {
      expect(useRequestStore.getState().loading).toBe(false);
    });

    it("starts with no error", () => {
      expect(useRequestStore.getState().error).toBeNull();
    });
  });

  // -----------------------------------------------------------------------
  // Method & URL
  // -----------------------------------------------------------------------

  describe("setMethod", () => {
    it("changes HTTP method", () => {
      useRequestStore.getState().setMethod("POST");
      expect(useRequestStore.getState().method).toBe("POST");
    });

    it("accepts all valid methods", () => {
      const methods = ["GET", "POST", "PUT", "DELETE", "PATCH", "HEAD", "OPTIONS"] as const;
      for (const m of methods) {
        useRequestStore.getState().setMethod(m);
        expect(useRequestStore.getState().method).toBe(m);
      }
    });
  });

  describe("setUrl", () => {
    it("sets the url", () => {
      useRequestStore.getState().setUrl("http://example.com");
      expect(useRequestStore.getState().url).toBe("http://example.com");
    });
  });

  // -----------------------------------------------------------------------
  // Headers
  // -----------------------------------------------------------------------

  describe("headers", () => {
    it("addHeader appends an empty header", () => {
      useRequestStore.getState().addHeader();
      const headers = useRequestStore.getState().headers;
      expect(headers).toHaveLength(1);
      expect(headers[0].key).toBe("");
      expect(headers[0].value).toBe("");
      expect(headers[0].id).toBeTruthy();
    });

    it("addHeader appends multiple headers", () => {
      useRequestStore.getState().addHeader();
      useRequestStore.getState().addHeader();
      expect(useRequestStore.getState().headers).toHaveLength(2);
    });

    it("updateHeader changes the correct header", () => {
      useRequestStore.getState().addHeader();
      const id = useRequestStore.getState().headers[0].id;
      useRequestStore.getState().updateHeader(id, "Content-Type", "application/json");
      const header = useRequestStore.getState().headers[0];
      expect(header.key).toBe("Content-Type");
      expect(header.value).toBe("application/json");
    });

    it("updateHeader does not affect other headers", () => {
      useRequestStore.getState().addHeader();
      useRequestStore.getState().addHeader();
      const headers = useRequestStore.getState().headers;
      useRequestStore.getState().updateHeader(headers[0].id, "X-First", "1");
      const updated = useRequestStore.getState().headers;
      expect(updated[1].key).toBe(""); // unchanged
    });

    it("removeHeader removes the correct header", () => {
      useRequestStore.getState().addHeader();
      useRequestStore.getState().addHeader();
      const idToRemove = useRequestStore.getState().headers[0].id;
      useRequestStore.getState().removeHeader(idToRemove);
      const remaining = useRequestStore.getState().headers;
      expect(remaining).toHaveLength(1);
      expect(remaining[0].id).not.toBe(idToRemove);
    });
  });

  // -----------------------------------------------------------------------
  // Body
  // -----------------------------------------------------------------------

  describe("body", () => {
    it("setBodyType changes body type", () => {
      useRequestStore.getState().setBodyType("json");
      expect(useRequestStore.getState().bodyType).toBe("json");
    });

    it("setBodyText changes body text", () => {
      useRequestStore.getState().setBodyText('{"key": "value"}');
      expect(useRequestStore.getState().bodyText).toBe('{"key": "value"}');
    });
  });

  // -----------------------------------------------------------------------
  // Form Data
  // -----------------------------------------------------------------------

  describe("form data", () => {
    it("addFormDataEntry appends empty entry", () => {
      useRequestStore.getState().addFormDataEntry();
      const entries = useRequestStore.getState().formDataEntries;
      expect(entries).toHaveLength(1);
      expect(entries[0].key).toBe("");
      expect(entries[0].value).toBe("");
    });

    it("updateFormDataEntry changes entry", () => {
      useRequestStore.getState().addFormDataEntry();
      const id = useRequestStore.getState().formDataEntries[0].id;
      useRequestStore.getState().updateFormDataEntry(id, "username", "admin");
      const entry = useRequestStore.getState().formDataEntries[0];
      expect(entry.key).toBe("username");
      expect(entry.value).toBe("admin");
    });

    it("removeFormDataEntry removes entry", () => {
      useRequestStore.getState().addFormDataEntry();
      useRequestStore.getState().addFormDataEntry();
      const id = useRequestStore.getState().formDataEntries[0].id;
      useRequestStore.getState().removeFormDataEntry(id);
      expect(useRequestStore.getState().formDataEntries).toHaveLength(1);
    });
  });

  // -----------------------------------------------------------------------
  // Auth
  // -----------------------------------------------------------------------

  describe("auth", () => {
    it("setAuth sets bearer auth", () => {
      useRequestStore.getState().setAuth({ type: "bearer", token: "abc" } as never);
      expect(useRequestStore.getState().auth).toBeTruthy();
    });

    it("setAuth clears auth with null", () => {
      useRequestStore.getState().setAuth({ type: "bearer", token: "abc" } as never);
      useRequestStore.getState().setAuth(null);
      expect(useRequestStore.getState().auth).toBeNull();
    });
  });

  // -----------------------------------------------------------------------
  // Query Params
  // -----------------------------------------------------------------------

  describe("query params", () => {
    it("addQueryParam adds a param", () => {
      useRequestStore.getState().setUrl("http://example.com");
      useRequestStore.getState().addQueryParam();
      expect(useRequestStore.getState().queryParams).toHaveLength(1);
      expect(useRequestStore.getState().queryParams[0].enabled).toBe(true);
    });

    it("updateQueryParam updates key and value", () => {
      useRequestStore.getState().setUrl("http://example.com");
      useRequestStore.getState().addQueryParam();
      const id = useRequestStore.getState().queryParams[0].id;
      useRequestStore.getState().updateQueryParam(id, "page", "2");
      const param = useRequestStore.getState().queryParams[0];
      expect(param.key).toBe("page");
      expect(param.value).toBe("2");
    });

    it("toggleQueryParam toggles enabled state", () => {
      useRequestStore.getState().setUrl("http://example.com");
      useRequestStore.getState().addQueryParam();
      const id = useRequestStore.getState().queryParams[0].id;
      expect(useRequestStore.getState().queryParams[0].enabled).toBe(true);
      useRequestStore.getState().toggleQueryParam(id);
      expect(useRequestStore.getState().queryParams[0].enabled).toBe(false);
      useRequestStore.getState().toggleQueryParam(id);
      expect(useRequestStore.getState().queryParams[0].enabled).toBe(true);
    });

    it("removeQueryParam removes param", () => {
      useRequestStore.getState().setUrl("http://example.com");
      useRequestStore.getState().addQueryParam();
      useRequestStore.getState().addQueryParam();
      const id = useRequestStore.getState().queryParams[0].id;
      useRequestStore.getState().removeQueryParam(id);
      expect(useRequestStore.getState().queryParams).toHaveLength(1);
    });
  });

  // -----------------------------------------------------------------------
  // syncUrlToParams
  // -----------------------------------------------------------------------

  describe("syncUrlToParams", () => {
    it("parses query params from URL", () => {
      useRequestStore.getState().syncUrlToParams("http://example.com?a=1&b=2");
      const params = useRequestStore.getState().queryParams;
      expect(params).toHaveLength(2);
      expect(params[0].key).toBe("a");
      expect(params[0].value).toBe("1");
      expect(params[1].key).toBe("b");
      expect(params[1].value).toBe("2");
    });

    it("sets url", () => {
      useRequestStore.getState().syncUrlToParams("http://example.com?x=y");
      expect(useRequestStore.getState().url).toBe("http://example.com?x=y");
    });

    it("handles URL without params", () => {
      useRequestStore.getState().syncUrlToParams("http://example.com");
      expect(useRequestStore.getState().queryParams).toEqual([]);
    });

    it("handles invalid URL", () => {
      useRequestStore.getState().syncUrlToParams("not-a-url");
      expect(useRequestStore.getState().queryParams).toEqual([]);
    });
  });

  // -----------------------------------------------------------------------
  // clearResponse
  // -----------------------------------------------------------------------

  describe("clearResponse", () => {
    it("clears response and error", () => {
      useRequestStore.setState({
        response: { status: 200, headers: {}, body: "ok", elapsed_ms: 10, size_bytes: 2 },
        error: "some error",
      });
      useRequestStore.getState().clearResponse();
      expect(useRequestStore.getState().response).toBeNull();
      expect(useRequestStore.getState().error).toBeNull();
    });
  });

  // -----------------------------------------------------------------------
  // loadFromHistory
  // -----------------------------------------------------------------------

  describe("loadFromHistory", () => {
    it("restores state from a GET history entry", () => {
      const entry: HistoryEntry = {
        id: "hist-1",
        input: {
          method: "POST",
          url: "http://api.example.com/users",
          headers: { "Content-Type": "application/json" },
          body: { type: "json", json: '{"name":"John"}' },
          auth: null,
        },
        output: {
          status: 201,
          headers: {},
          body: '{"id": 1}',
          elapsed_ms: 55,
          size_bytes: 9,
        },
        timestamp: "2026-01-01T00:00:00Z",
      };

      useRequestStore.getState().loadFromHistory(entry);
      const state = useRequestStore.getState();
      expect(state.method).toBe("POST");
      expect(state.url).toBe("http://api.example.com/users");
      expect(state.bodyType).toBe("json");
      expect(state.bodyText).toBe('{"name":"John"}');
      expect(state.headers).toHaveLength(1);
      expect(state.headers[0].key).toBe("Content-Type");
      expect(state.headers[0].value).toBe("application/json");
      expect(state.response).toBeNull();
      expect(state.error).toBeNull();
    });

    it("restores form_data body type", () => {
      const entry: HistoryEntry = {
        id: "hist-2",
        input: {
          method: "POST",
          url: "http://example.com",
          headers: {},
          body: { type: "form_data", form_data: [["key1", "val1"], ["key2", "val2"]] },
          auth: null,
        },
        output: { status: 200, headers: {}, body: "", elapsed_ms: 10, size_bytes: 0 },
        timestamp: "2026-01-01T00:00:00Z",
      };

      useRequestStore.getState().loadFromHistory(entry);
      const state = useRequestStore.getState();
      expect(state.bodyType).toBe("form_data");
      expect(state.formDataEntries).toHaveLength(2);
      expect(state.formDataEntries[0].key).toBe("key1");
      expect(state.formDataEntries[0].value).toBe("val1");
    });

    it("restores raw body type", () => {
      const entry: HistoryEntry = {
        id: "hist-3",
        input: {
          method: "POST",
          url: "http://example.com",
          headers: {},
          body: { type: "raw", raw: "raw text" },
          auth: null,
        },
        output: { status: 200, headers: {}, body: "", elapsed_ms: 10, size_bytes: 0 },
        timestamp: "2026-01-01T00:00:00Z",
      };

      useRequestStore.getState().loadFromHistory(entry);
      expect(useRequestStore.getState().bodyType).toBe("raw");
      expect(useRequestStore.getState().bodyText).toBe("raw text");
    });

    it("restores xml body type", () => {
      const entry: HistoryEntry = {
        id: "hist-4",
        input: {
          method: "POST",
          url: "http://example.com",
          headers: {},
          body: { type: "xml", xml: "<root/>" },
          auth: null,
        },
        output: { status: 200, headers: {}, body: "", elapsed_ms: 10, size_bytes: 0 },
        timestamp: "2026-01-01T00:00:00Z",
      };

      useRequestStore.getState().loadFromHistory(entry);
      expect(useRequestStore.getState().bodyType).toBe("xml");
      expect(useRequestStore.getState().bodyText).toBe("<root/>");
    });

    it("restores no-body request", () => {
      const entry: HistoryEntry = {
        id: "hist-5",
        input: {
          method: "GET",
          url: "http://example.com",
          headers: {},
          body: null,
          auth: null,
        },
        output: { status: 200, headers: {}, body: "", elapsed_ms: 10, size_bytes: 0 },
        timestamp: "2026-01-01T00:00:00Z",
      };

      useRequestStore.getState().loadFromHistory(entry);
      expect(useRequestStore.getState().bodyType).toBe("none");
      expect(useRequestStore.getState().bodyText).toBe("");
    });
  });
});
