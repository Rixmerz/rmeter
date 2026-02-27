import { useState, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/cn";
import type { ThreadGroup, LoopCount } from "@/types/plan";
import { usePlanStore } from "@/stores/usePlanStore";

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

const inputClass = cn(
  "w-full text-sm px-2 py-1.5 rounded border border-input bg-background",
  "focus:outline-none focus:ring-1 focus:ring-ring"
);

interface LoopCountEditorProps {
  value: LoopCount;
  onChange: (lc: LoopCount) => void;
}

function LoopCountEditor({ value, onChange }: LoopCountEditorProps) {
  return (
    <div className="space-y-2">
      <div className="flex gap-2">
        {(["finite", "duration", "infinite"] as const).map((type) => (
          <button
            key={type}
            onClick={() => {
              if (type === "finite") onChange({ type: "finite", count: 1 });
              else if (type === "duration") onChange({ type: "duration", seconds: 60 });
              else onChange({ type: "infinite" });
            }}
            className={cn(
              "flex-1 py-1 text-xs rounded border transition-colors",
              "focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring",
              value.type === type
                ? "bg-primary text-primary-foreground border-primary"
                : "border-input hover:bg-accent hover:text-accent-foreground"
            )}
          >
            {type === "finite" ? "Count" : type === "duration" ? "Duration" : "Infinite"}
          </button>
        ))}
      </div>
      {value.type === "finite" && (
        <div className="space-y-1">
          <label htmlFor="loop-count" className="text-xs text-muted-foreground">
            Iterations
          </label>
          <input
            id="loop-count"
            type="number"
            min={1}
            value={value.count}
            onChange={(e) => onChange({ type: "finite", count: Math.max(1, parseInt(e.target.value, 10) || 1) })}
            className={inputClass}
          />
        </div>
      )}
      {value.type === "duration" && (
        <div className="space-y-1">
          <label htmlFor="loop-duration" className="text-xs text-muted-foreground">
            Duration (seconds)
          </label>
          <input
            id="loop-duration"
            type="number"
            min={1}
            value={value.seconds}
            onChange={(e) => onChange({ type: "duration", seconds: Math.max(1, parseInt(e.target.value, 10) || 1) })}
            className={inputClass}
          />
        </div>
      )}
    </div>
  );
}

interface ThreadGroupPropertiesProps {
  group: ThreadGroup;
}

export function ThreadGroupProperties({ group }: ThreadGroupPropertiesProps) {
  const { updateThreadGroup } = usePlanStore();

  const [name, setName] = useState(group.name);
  const [numThreads, setNumThreads] = useState(group.num_threads);
  const [rampUp, setRampUp] = useState(group.ramp_up_seconds);
  const [loopCount, setLoopCount] = useState<LoopCount>(group.loop_count);
  const [dirty, setDirty] = useState(false);

  // Sync state when selected group changes
  useEffect(() => {
    setName(group.name);
    setNumThreads(group.num_threads);
    setRampUp(group.ramp_up_seconds);
    setLoopCount(group.loop_count);
    setDirty(false);
  }, [group.id, group.name, group.num_threads, group.ramp_up_seconds, group.loop_count]);

  function markDirty() {
    setDirty(true);
  }

  async function handleSave() {
    await updateThreadGroup(group.id, {
      name: name.trim() || group.name,
      num_threads: numThreads,
      ramp_up_seconds: rampUp,
      loop_count: loopCount,
    });
    setDirty(false);
  }

  function handleDiscard() {
    setName(group.name);
    setNumThreads(group.num_threads);
    setRampUp(group.ramp_up_seconds);
    setLoopCount(group.loop_count);
    setDirty(false);
  }

  return (
    <div className="space-y-4">
      <h3 className="text-sm font-semibold">Thread Group</h3>

      <Field label="Name" htmlFor="tg-name">
        <input
          id="tg-name"
          type="text"
          value={name}
          onChange={(e) => { setName(e.target.value); markDirty(); }}
          className={inputClass}
        />
      </Field>

      <Field label="Virtual Users (threads)" htmlFor="tg-threads">
        <input
          id="tg-threads"
          type="number"
          min={1}
          max={10000}
          value={numThreads}
          onChange={(e) => { setNumThreads(Math.max(1, parseInt(e.target.value, 10) || 1)); markDirty(); }}
          className={inputClass}
        />
      </Field>

      <Field label="Ramp-up (seconds)" htmlFor="tg-rampup">
        <input
          id="tg-rampup"
          type="number"
          min={0}
          value={rampUp}
          onChange={(e) => { setRampUp(Math.max(0, parseInt(e.target.value, 10) || 0)); markDirty(); }}
          className={inputClass}
        />
        <p className="text-xs text-muted-foreground mt-1">
          Time to gradually start all virtual users.
        </p>
      </Field>

      <Field label="Loop Mode" htmlFor="loop-count">
        <LoopCountEditor
          value={loopCount}
          onChange={(lc) => { setLoopCount(lc); markDirty(); }}
        />
      </Field>

      <Field label="Status" htmlFor="tg-enabled">
        <div className="flex items-center gap-2">
          <input
            id="tg-enabled"
            type="checkbox"
            checked={group.enabled}
            readOnly
            className="h-4 w-4 rounded border border-input"
            aria-label="Enabled"
          />
          <span className="text-sm">{group.enabled ? "Enabled" : "Disabled"}</span>
          <span className="text-xs text-muted-foreground">(use tree context menu to toggle)</span>
        </div>
      </Field>

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
