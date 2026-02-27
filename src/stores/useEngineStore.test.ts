import { describe, it, expect, beforeEach, vi } from "vitest";
import { useEngineStore } from "./useEngineStore";
import type { ProgressEvent } from "@/types/engine";
import type { TestSummary, RequestResultEvent } from "@/types/results";

// Mock the commands module
vi.mock("@/lib/commands", () => ({
  startTest: vi.fn(),
  stopTest: vi.fn(),
  forceStopTest: vi.fn(),
}));

function makeProgressEvent(overrides?: Partial<ProgressEvent>): ProgressEvent {
  return {
    completed_requests: 100,
    total_errors: 2,
    active_threads: 10,
    elapsed_ms: 5000,
    current_rps: 20.0,
    mean_ms: 45.5,
    p95_ms: 120,
    min_ms: 5,
    max_ms: 500,
    ...overrides,
  };
}

function makeResultEvent(overrides?: Partial<RequestResultEvent>): RequestResultEvent {
  return {
    id: "result-1",
    plan_id: "plan-1",
    thread_group_name: "Group 1",
    request_name: "GET /api",
    timestamp: "2026-01-01T00:00:00Z",
    status_code: 200,
    elapsed_ms: 42,
    size_bytes: 1024,
    assertions_passed: true,
    error: null,
    assertion_results: [],
    extraction_results: [],
    method: "GET",
    url: "http://example.com/api",
    ...overrides,
  };
}

function makeSummary(overrides?: Partial<TestSummary>): TestSummary {
  return {
    plan_id: "plan-1",
    plan_name: "Test Plan",
    total_requests: 100,
    successful_requests: 98,
    failed_requests: 2,
    min_response_ms: 5,
    max_response_ms: 500,
    mean_response_ms: 45.5,
    p50_response_ms: 40,
    p95_response_ms: 120,
    p99_response_ms: 200,
    requests_per_second: 20.0,
    total_bytes_received: 102400,
    started_at: "2026-01-01T00:00:00Z",
    finished_at: "2026-01-01T00:00:05Z",
    ...overrides,
  };
}

describe("useEngineStore", () => {
  beforeEach(() => {
    useEngineStore.getState().reset();
  });

  describe("initial state", () => {
    it("starts with idle status", () => {
      const state = useEngineStore.getState();
      expect(state.status).toBe("idle");
      expect(state.progress).toBeNull();
      expect(state.lastSummary).toBeNull();
      expect(state.recentResults).toEqual([]);
      expect(state.chartData).toEqual([]);
      expect(state.error).toBeNull();
      expect(state.stoppingAt).toBeNull();
    });
  });

  describe("reset", () => {
    it("clears all state back to initial values", () => {
      // Dirty the state
      useEngineStore.getState().onStatusChange("running");
      useEngineStore.getState().onProgress(makeProgressEvent());
      useEngineStore.getState().onResult(makeResultEvent());

      // Reset
      useEngineStore.getState().reset();
      const state = useEngineStore.getState();
      expect(state.status).toBe("idle");
      expect(state.progress).toBeNull();
      expect(state.recentResults).toEqual([]);
      expect(state.chartData).toEqual([]);
    });
  });

  describe("onProgress", () => {
    it("updates progress and appends chart data point", () => {
      const event = makeProgressEvent({ elapsed_ms: 1000, current_rps: 15.0 });
      useEngineStore.getState().onProgress(event);

      const state = useEngineStore.getState();
      expect(state.progress).toEqual(event);
      expect(state.chartData).toHaveLength(1);
      expect(state.chartData[0].elapsed_s).toBe(1);
      expect(state.chartData[0].rps).toBe(15.0);
    });

    it("accumulates multiple chart data points", () => {
      useEngineStore.getState().onProgress(makeProgressEvent({ elapsed_ms: 1000 }));
      useEngineStore.getState().onProgress(makeProgressEvent({ elapsed_ms: 2000 }));
      useEngineStore.getState().onProgress(makeProgressEvent({ elapsed_ms: 3000 }));

      expect(useEngineStore.getState().chartData).toHaveLength(3);
    });

    it("calculates error rate correctly", () => {
      useEngineStore.getState().onProgress(
        makeProgressEvent({ completed_requests: 200, total_errors: 10 })
      );

      const point = useEngineStore.getState().chartData[0];
      expect(point.error_rate).toBe(5); // 10/200 * 100
    });

    it("handles zero completed requests without division by zero", () => {
      useEngineStore.getState().onProgress(
        makeProgressEvent({ completed_requests: 0, total_errors: 0 })
      );

      const point = useEngineStore.getState().chartData[0];
      expect(point.error_rate).toBe(0);
    });
  });

  describe("onResult", () => {
    it("appends result to recentResults", () => {
      useEngineStore.getState().onResult(makeResultEvent());
      expect(useEngineStore.getState().recentResults).toHaveLength(1);
    });

    it("caps recentResults at 50 entries", () => {
      for (let i = 0; i < 60; i++) {
        useEngineStore.getState().onResult(
          makeResultEvent({ request_name: `Request ${i}` })
        );
      }
      const results = useEngineStore.getState().recentResults;
      expect(results).toHaveLength(50);
      // Should keep the most recent entries
      expect(results[results.length - 1].request_name).toBe("Request 59");
    });
  });

  describe("onStatusChange", () => {
    it("updates status", () => {
      useEngineStore.getState().onStatusChange("running");
      expect(useEngineStore.getState().status).toBe("running");
    });

    it("sets stoppingAt when entering stopping state", () => {
      const before = Date.now();
      useEngineStore.getState().onStatusChange("stopping");
      const after = Date.now();

      const stoppingAt = useEngineStore.getState().stoppingAt;
      expect(stoppingAt).not.toBeNull();
      expect(stoppingAt!).toBeGreaterThanOrEqual(before);
      expect(stoppingAt!).toBeLessThanOrEqual(after);
    });

    it("clears stoppingAt when leaving stopping state", () => {
      useEngineStore.getState().onStatusChange("stopping");
      expect(useEngineStore.getState().stoppingAt).not.toBeNull();

      useEngineStore.getState().onStatusChange("completed");
      expect(useEngineStore.getState().stoppingAt).toBeNull();
    });

    it("clears error on status change", () => {
      useEngineStore.setState({ error: "some error" });
      useEngineStore.getState().onStatusChange("running");
      expect(useEngineStore.getState().error).toBeNull();
    });
  });

  describe("onStatusEvent", () => {
    it("delegates to onStatusChange", () => {
      useEngineStore.getState().onStatusEvent({ status: "running" });
      expect(useEngineStore.getState().status).toBe("running");
    });
  });

  describe("onComplete", () => {
    it("stores summary and sets status to completed", () => {
      const summary = makeSummary({ plan_name: "My Plan" });
      useEngineStore.getState().onComplete(summary);

      const state = useEngineStore.getState();
      expect(state.lastSummary).toEqual(summary);
      expect(state.status).toBe("completed");
      expect(state.stoppingAt).toBeNull();
    });
  });
});
