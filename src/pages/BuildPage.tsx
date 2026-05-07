import { invoke } from "@tauri-apps/api/core";
import { Loader2, Play, Square } from "lucide-react";
import { useCallback, useState } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { BuildOptionsForm } from "@/components/pipeline/BuildOptionsForm";
import { BuildResultCard } from "@/components/pipeline/BuildResultCard";
import { EngineDetectBadge } from "@/components/pipeline/EngineDetectBadge";
import { GameDropZone } from "@/components/pipeline/GameDropZone";
import { LogTerminal } from "@/components/pipeline/LogTerminal";
import { PipelineStepper } from "@/components/pipeline/PipelineStepper";
import { usePipelineContext } from "@/contexts/PipelineContext";
import { useSettings } from "@/hooks/useSettings";
import { BuildOptions, EngineDetectResult, ValidationResult } from "@/types/pipeline";

type PageState = "idle" | "detecting" | "ready" | "preflight" | "password";

interface PreflightIssue {
  severity: "error" | "warning";
  message: string;
  hint: string;
}

export function BuildPage() {
  const { settings } = useSettings();
  const { buildState, stages, logs, result, error, startBuild, cancelBuild, reset } =
    usePipelineContext();

  const [pageState, setPageState] = useState<PageState>("idle");
  const [gamePath, setGamePath] = useState("");
  const [detection, setDetection] = useState<EngineDetectResult | null>(null);
  const [validation, setValidation] = useState<ValidationResult | null>(null);
  const [options, setOptions] = useState<Partial<BuildOptions>>({
    version_name: "1.0.0",
    version_code: 1,
  });
  const [storepass, setStorepass] = useState("");
  const [keypass, setKeypass] = useState("");
  const [preflightIssues, setPreflightIssues] = useState<PreflightIssue[]>([]);

  // When pipeline finishes/fails, keep showing the terminal & result on the build page
  // but don't reset pageState — user stays on the result view.
  const isBuilding = buildState === "building";
  const isDone = buildState === "done";
  const isFailed = buildState === "failed";
  const showPipeline = isBuilding || isDone || isFailed;

  const onFolderSelected = useCallback(
    async (path: string) => {
      setGamePath(path);
      setDetection(null);
      setValidation(null);
      setPageState("detecting");

      try {
        const [det, val] = await Promise.all([
          invoke<EngineDetectResult>("cmd_detect_engine", { gamePath: path }),
          invoke<ValidationResult>("cmd_validate_game_folder", { gamePath: path }),
        ]);

        setDetection(det);
        setValidation(val);

        const folderName = path.split("/").pop() ?? "Game";
        const appId = await invoke<string>("cmd_suggest_app_id", { folderName });
        setOptions((prev) => ({
          ...prev,
          app_id: appId,
          app_name: folderName,
          engine: det.engine,
        }));

        setPageState("ready");
      } catch (e) {
        toast.error(`Detection failed: ${e}`);
        setPageState("idle");
      }
    },
    []
  );

  const handleBuild = useCallback(async () => {
    if (!settings.keystore_path) {
      toast.error("No keystore configured. Set it up in Settings first.");
      return;
    }
    try {
      const issues = await invoke<PreflightIssue[]>("cmd_preflight_check", {
        engine: options.engine ?? detection?.engine ?? "",
        gamePath,
      });
      setPreflightIssues(issues);
      const hasErrors = issues.some((i) => i.severity === "error");
      if (hasErrors) {
        setPageState("preflight");
      } else {
        setPageState("password");
      }
    } catch {
      setPageState("password"); // preflight unavailable — proceed anyway
    }
  }, [settings.keystore_path, options.engine, detection, gamePath]);

  const confirmBuild = useCallback(async () => {
    const full: BuildOptions = {
      app_id: options.app_id ?? "com.example.game",
      app_name: options.app_name ?? "Game",
      version_name: options.version_name ?? "1.0.0",
      version_code: options.version_code ?? 1,
      icon_path: options.icon_path ?? null,
      engine: options.engine ?? detection?.engine ?? "",
      storepass,
      keypass,
    };
    setStorepass("");
    setKeypass("");
    setPageState("idle"); // close dialog, pipeline state tracked globally
    await startBuild(gamePath, full);
  }, [options, gamePath, storepass, keypass, startBuild]);

  const handleReset = useCallback(() => {
    reset();
    setPageState("idle");
    setGamePath("");
    setDetection(null);
    setValidation(null);
  }, [reset]);

  const engineSupported =
    detection?.engine === "RpgMakerMV" ||
    detection?.engine === "RpgMakerMZ" ||
    detection?.engine === "TyranoBuilder" ||
    detection?.engine === "RenPy";

  // If a build is already in progress from a previous page visit, show the pipeline
  const showDropZone = !showPipeline && (pageState === "idle" || pageState === "detecting");
  const showGameCard =
    !showPipeline && (pageState === "ready") && detection;

  return (
    <div className="flex flex-col gap-4 p-6 overflow-y-auto h-full">
      <div>
        <h1 className="text-xl font-semibold">Build APK</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Drop a game folder and convert it to a signed Android APK.
        </p>
      </div>

      {/* Drop zone */}
      {showDropZone && (
        <GameDropZone onFolderSelected={onFolderSelected} disabled={pageState === "detecting"} />
      )}

      {pageState === "detecting" && !showPipeline && (
        <div className="flex items-center gap-2 text-sm text-muted-foreground">
          <Loader2 className="h-4 w-4 animate-spin" />
          Detecting engine…
        </div>
      )}

      {/* Game info + build options (only before build starts) */}
      {showGameCard && (
        <Card>
          <CardHeader className="pb-3">
            <CardTitle className="text-sm font-medium text-muted-foreground truncate">
              {gamePath}
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <EngineDetectBadge result={detection} />

            {validation && validation.issues.length > 0 && (
              <div className="space-y-1">
                {validation.issues.map((issue, i) => (
                  <p key={i} className="text-xs text-amber-400">⚠ {issue}</p>
                ))}
              </div>
            )}

            <Separator />

            <BuildOptionsForm options={options} onChange={setOptions} disabled={false} />

            <div className="flex justify-end pt-1">
              <Button
                onClick={handleBuild}
                disabled={!engineSupported || !validation?.valid}
                className="gap-2"
              >
                <Play className="h-4 w-4" />
                Build APK
              </Button>
            </div>
          </CardContent>
        </Card>
      )}

      {/* Pipeline progress — persists across navigation */}
      {showPipeline && (
        <>
          <Card>
            <CardContent className="pt-4 space-y-4">
              <div className="flex items-center justify-between">
                <p className="text-xs text-muted-foreground truncate max-w-sm">{gamePath}</p>
                {isBuilding && (
                  <Button variant="destructive" size="sm" onClick={cancelBuild} className="gap-1">
                    <Square className="h-3 w-3" />
                    Cancel
                  </Button>
                )}
              </div>
              <PipelineStepper stages={stages} />
            </CardContent>
          </Card>

          <LogTerminal logs={logs} />

          <BuildResultCard result={result} error={error} onReset={handleReset} />
        </>
      )}

      {/* Pre-flight issues dialog */}
      <Dialog open={pageState === "preflight"} onOpenChange={(o) => !o && setPageState("ready")}>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Fix issues before building</DialogTitle>
          </DialogHeader>
          <div className="space-y-2 max-h-72 overflow-y-auto">
            {preflightIssues.map((issue, i) => (
              <div key={i} className={`rounded-md border p-3 text-sm space-y-1 ${issue.severity === "error" ? "border-destructive/50 bg-destructive/10" : "border-amber-500/50 bg-amber-500/10"}`}>
                <p className={issue.severity === "error" ? "text-destructive font-medium" : "text-amber-400 font-medium"}>
                  {issue.severity === "error" ? "✗" : "⚠"} {issue.message}
                </p>
                {issue.hint && <p className="text-xs text-muted-foreground">{issue.hint}</p>}
              </div>
            ))}
          </div>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setPageState("ready")}>Cancel</Button>
            <Button
              onClick={() => setPageState("password")}
              disabled={preflightIssues.some((i) => i.severity === "error")}
            >
              Build Anyway
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Keystore password dialog */}
      <Dialog open={pageState === "password"} onOpenChange={(o) => !o && setPageState("ready")}>
        <DialogContent className="max-w-sm">
          <DialogHeader>
            <DialogTitle>Keystore Passwords</DialogTitle>
          </DialogHeader>
          <div className="space-y-3">
            <p className="text-xs text-muted-foreground">
              Keystore: <code className="text-xs">{settings.keystore_path}</code>
            </p>
            <div className="space-y-1">
              <Label>Store Password</Label>
              <Input
                type="password"
                value={storepass}
                onChange={(e) => setStorepass(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && keypass && confirmBuild()}
                autoFocus
              />
            </div>
            <div className="space-y-1">
              <Label>Key Password</Label>
              <Input
                type="password"
                value={keypass}
                onChange={(e) => setKeypass(e.target.value)}
                onKeyDown={(e) => e.key === "Enter" && storepass && confirmBuild()}
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="ghost" onClick={() => setPageState("ready")}>Cancel</Button>
            <Button onClick={confirmBuild} disabled={!storepass || !keypass} className="gap-2">
              <Play className="h-4 w-4" />
              Start Build
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
