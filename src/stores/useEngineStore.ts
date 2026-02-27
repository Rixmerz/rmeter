import { create } from "zustand";
import type { EngineStatusKind, ProgressEvent, StatusChangeEvent } from "@/types/engine";
import type { TestSummary, RequestResultEvent } from "@/types/results";
import {
  startTest as startTestCmd,
  stopTest as stopTestCmd,
  forceStopTest as forceStopTestCmd,
} from "@/lib/commands";

const MAX_RECENT_RESULTS = 50;

export interface ChartDataPoint {
  elapsed_s: number;       // seconds since test start
  rps: number;
  completed: number;
  errors: number;
  error_rate: number;      // percentage
  active_threads: number;
  mean_ms: number;
  p95_ms: number;
}

interface EngineState {
  status: EngineStatusKind;
  progress: ProgressEvent | null;
  lastSummary: TestSummary | null;
  recentResults: RequestResultEvent[];
  chartData: ChartDataPoint[];
  error: string | null;
  /** Timestamp (ms) when status entered "stopping" — used to show force stop */
  stoppingAt: number | null;
}

interface EngineActions {
  startTest(planId: string): Promise<void>;
  stopTest(): Promise<void>;
  forceStopTest(): Promise<void>;
  reset(): void;
  // Called by event listeners:
  onProgress(event: ProgressEvent): void;
  onResult(event: RequestResultEvent): void;
  onStatusChange(status: EngineStatusKind): void;
  onComplete(summary: TestSummary): void;
  // Internal helper exposed for StatusChangeEvent shape:
  onStatusEvent(event: StatusChangeEvent): void;
}

export const useEngineStore = create<EngineState & EngineActions>((set, get) => ({
  status: "idle",
  progress: null,
  lastSummary: null,
  recentResults: [],
  chartData: [],
  error: null,
  stoppingAt: null,

  startTest: async (planId) => {
    set({ error: null });
    try {
      await startTestCmd(planId);
      // Optimistically set running; real status arrives via event
      set({ status: "running", progress: null, recentResults: [], chartData: [] });
    } catch (err) {
      set({
        error: err instanceof Error ? err.message : String(err),
        status: "error",
      });
    }
  },

  stopTest: async () => {
    set({ error: null });
    try {
      await stopTestCmd();
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  forceStopTest: async () => {
    set({ error: null });
    try {
      await forceStopTestCmd();
    } catch (err) {
      set({ error: err instanceof Error ? err.message : String(err) });
    }
  },

  reset: () => {
    set({
      status: "idle",
      progress: null,
      lastSummary: null,
      recentResults: [],
      chartData: [],
      error: null,
      stoppingAt: null,
    });
  },

  onProgress: (event) => {
    const point: ChartDataPoint = {
      elapsed_s: Math.round(event.elapsed_ms / 1000),
      rps: event.current_rps,
      completed: event.completed_requests,
      errors: event.total_errors,
      error_rate:
        event.completed_requests > 0
          ? (event.total_errors / event.completed_requests) * 100
          : 0,
      active_threads: event.active_threads,
      mean_ms: event.mean_ms ?? 0,
      p95_ms: event.p95_ms ?? 0,
    };
    set((state) => ({
      progress: event,
      chartData: [...state.chartData, point],
    }));
  },

  onResult: (event) => {
    set((state) => {
      const updated = [...state.recentResults, event];
      // Keep only the last MAX_RECENT_RESULTS entries
      if (updated.length > MAX_RECENT_RESULTS) {
        return { recentResults: updated.slice(updated.length - MAX_RECENT_RESULTS) };
      }
      return { recentResults: updated };
    });
  },

  onStatusChange: (status) => {
    const prev = get().status;
    set({
      status,
      stoppingAt: status === "stopping" && prev !== "stopping" ? Date.now() : get().stoppingAt,
      error: null,
    });
    // Clear stoppingAt when no longer stopping
    if (status !== "stopping") {
      set({ stoppingAt: null });
    }
  },

  onStatusEvent: (event) => {
    get().onStatusChange(event.status);
  },

  onComplete: (summary) => {
    set({
      lastSummary: summary,
      status: "completed",
      stoppingAt: null,
    });
  },
}));
