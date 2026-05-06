import { Check, Loader2, X } from "lucide-react";
import { StageState } from "@/contexts/PipelineContext";
import { PipelineStage } from "@/types/pipeline";
import { cn } from "@/lib/utils";

const STAGE_LABELS: Record<PipelineStage, string> = {
  Validate: "Validate",
  Scaffold: "Scaffold",
  CopyAssets: "Assets",
  PatchConfig: "Config",
  AddPlatform: "Platform",
  Build: "Build",
  Align: "Align",
  Sign: "Sign",
  Output: "Output",
};

interface Props {
  stages: StageState[];
}

export function PipelineStepper({ stages }: Props) {
  return (
    <div className="w-full overflow-x-auto">
      <div className="flex items-start min-w-max px-1">
        {stages.map((s, i) => {
          const isDone = s.status === "Done";
          const isRunning = s.status === "Running";
          const isFailed = s.status === "Failed";
          const isPending = s.status === "Pending";
          const nextDone = i < stages.length - 1 && stages[i + 1].status !== "Pending";

          return (
            <div key={s.stage} className="flex items-start">
              {/* Step node */}
              <div className="flex flex-col items-center gap-1.5 w-14">
                <div
                  className={cn(
                    "flex h-7 w-7 shrink-0 items-center justify-center rounded-full border-2 transition-all duration-300",
                    isDone && "border-emerald-500 bg-emerald-500/15",
                    isRunning && "border-primary bg-primary/15",
                    isFailed && "border-destructive bg-destructive/15",
                    isPending && "border-muted-foreground/30 bg-muted"
                  )}
                >
                  {isDone && <Check className="h-3.5 w-3.5 text-emerald-500" />}
                  {isRunning && <Loader2 className="h-3.5 w-3.5 text-primary animate-spin" />}
                  {isFailed && <X className="h-3.5 w-3.5 text-destructive" />}
                  {isPending && <span className="h-1.5 w-1.5 rounded-full bg-muted-foreground/30" />}
                </div>
                <span
                  className={cn(
                    "text-[9px] font-medium text-center leading-tight",
                    isDone && "text-emerald-500",
                    isRunning && "text-primary",
                    isFailed && "text-destructive",
                    isPending && "text-muted-foreground/50"
                  )}
                >
                  {STAGE_LABELS[s.stage]}
                </span>
              </div>

              {/* Connector — aligned to circle center (h-7 = 28px, so mt-[13px] = 28/2 - 1) */}
              {i < stages.length - 1 && (
                <div className="mt-[13px] flex-1 w-4 shrink-0">
                  <div
                    className={cn(
                      "h-[2px] w-full transition-colors duration-500",
                      isDone || nextDone ? "bg-emerald-500/60" : "bg-muted-foreground/20"
                    )}
                  />
                </div>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
