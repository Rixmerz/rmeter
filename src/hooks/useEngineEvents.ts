import { useTauriEvent } from "@/hooks/useTauriEvent";
import { useEngineStore } from "@/stores/useEngineStore";
import type { ProgressEvent, StatusChangeEvent } from "@/types/engine";
import type { TestSummary, RequestResultEvent } from "@/types/results";

/**
 * Subscribes to all Tauri engine events and routes them to the engine store.
 * Call this hook once in App.tsx so events are always captured.
 */
export function useEngineEvents() {
  const onProgress = useEngineStore((s) => s.onProgress);
  const onResult = useEngineStore((s) => s.onResult);
  const onStatusEvent = useEngineStore((s) => s.onStatusEvent);
  const onComplete = useEngineStore((s) => s.onComplete);

  useTauriEvent<ProgressEvent>("test-progress", onProgress);
  useTauriEvent<RequestResultEvent>("test-result", onResult);
  useTauriEvent<StatusChangeEvent>("test-status", onStatusEvent);
  useTauriEvent<TestSummary>("test-complete", onComplete);
}
