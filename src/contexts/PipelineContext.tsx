import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { createContext, useCallback, useContext, useEffect, useRef, useState } from "react";
import {
  BuildOptions,
  BuildResult,
  LogLine,
  PipelineStage,
  PipelineStageEvent,
  StageStatus,
} from "@/types/pipeline";

export type BuildState = "idle" | "building" | "done" | "failed";

export interface StageState {
  stage: PipelineStage;
  status: StageStatus;
}

const ORDERED_STAGES: PipelineStage[] = [
  "Validate",
  "Scaffold",
  "CopyAssets",
  "PatchConfig",
  "AddPlatform",
  "Build",
  "Align",
  "Sign",
  "Output",
];

const initialStages = (): StageState[] =>
  ORDERED_STAGES.map((stage) => ({ stage, status: "Pending" }));

interface PipelineContextValue {
  buildState: BuildState;
  stages: StageState[];
  logs: LogLine[];
  result: BuildResult | null;
  error: string | null;
  startBuild: (gamePath: string, options: BuildOptions) => Promise<void>;
  cancelBuild: () => Promise<void>;
  reset: () => void;
}

const PipelineContext = createContext<PipelineContextValue | null>(null);

export function PipelineProvider({ children }: { children: React.ReactNode }) {
  const [buildState, setBuildState] = useState<BuildState>("idle");
  const [stages, setStages] = useState<StageState[]>(initialStages());
  const [logs, setLogs] = useState<LogLine[]>([]);
  const [result, setResult] = useState<BuildResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const unlisteners = useRef<Array<() => void>>([]);

  // Clean up listeners — called on reset or when build finishes
  const cleanup = useCallback(() => {
    unlisteners.current.forEach((fn) => fn());
    unlisteners.current = [];
  }, []);

  // Only clean up on full unmount (app close), NOT on page navigation
  useEffect(() => () => cleanup(), [cleanup]);

  const startBuild = useCallback(
    async (gamePath: string, options: BuildOptions) => {
      cleanup();
      setBuildState("building");
      setStages(initialStages());
      setLogs([]);
      setResult(null);
      setError(null);

      const ul1 = await listen<PipelineStageEvent>("pipeline-stage", (e) => {
        setStages((prev) =>
          prev.map((s) =>
            s.stage === e.payload.stage ? { ...s, status: e.payload.status } : s
          )
        );
      });

      const ul2 = await listen<LogLine>("pipeline-log", (e) => {
        setLogs((prev) => [...prev, e.payload]);
      });

      const ul3 = await listen<BuildResult>("pipeline-done", (e) => {
        setResult(e.payload);
        setBuildState("done");
        cleanup();
      });

      const ul4 = await listen<string>("pipeline-error", (e) => {
        setError(e.payload);
        setBuildState("failed");
        cleanup();
      });

      unlisteners.current = [ul1, ul2, ul3, ul4];

      try {
        await invoke("cmd_start_build", { gamePath, buildOptions: options });
      } catch (e) {
        setError(String(e));
        setBuildState("failed");
        cleanup();
      }
    },
    [cleanup]
  );

  const cancelBuild = useCallback(async () => {
    await invoke("cmd_cancel_build").catch(() => {});
  }, []);

  const reset = useCallback(() => {
    cleanup();
    setBuildState("idle");
    setStages(initialStages());
    setLogs([]);
    setResult(null);
    setError(null);
  }, [cleanup]);

  return (
    <PipelineContext.Provider
      value={{ buildState, stages, logs, result, error, startBuild, cancelBuild, reset }}
    >
      {children}
    </PipelineContext.Provider>
  );
}

export function usePipelineContext() {
  const ctx = useContext(PipelineContext);
  if (!ctx) throw new Error("usePipelineContext must be used inside PipelineProvider");
  return ctx;
}
