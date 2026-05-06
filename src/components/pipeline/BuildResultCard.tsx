import { invoke } from "@tauri-apps/api/core";
import { CheckCircle2, Copy, FolderOpen, XCircle } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { BuildResult } from "@/types/pipeline";

interface Props {
  result?: BuildResult | null;
  error?: string | null;
  onReset: () => void;
}

export function BuildResultCard({ result, error, onReset }: Props) {
  if (!result && !error) return null;

  const openFolder = async (path: string) => {
    const dir = path.substring(0, path.lastIndexOf("/"));
    await invoke("cmd_open_file_manager", { path: dir }).catch(() => {});
  };

  const copyPath = async (path: string) => {
    await navigator.clipboard.writeText(path);
    toast.success("Path copied");
  };

  if (error) {
    return (
      <Card className="border-destructive/50 bg-destructive/5">
        <CardContent className="pt-5 space-y-3">
          <div className="flex items-center gap-2">
            <XCircle className="h-5 w-5 text-destructive shrink-0" />
            <span className="text-sm font-medium text-destructive">Build Failed</span>
          </div>
          <p className="text-xs font-mono text-muted-foreground break-all bg-black/30 rounded p-2">
            {error}
          </p>
          <Button variant="outline" size="sm" onClick={onReset}>
            Try Again
          </Button>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className="border-emerald-500/30 bg-emerald-500/5">
      <CardContent className="pt-5 space-y-3">
        <div className="flex items-center gap-2">
          <CheckCircle2 className="h-5 w-5 text-emerald-500 shrink-0" />
          <span className="text-sm font-medium text-emerald-400">Build Successful</span>
        </div>
        <p className="text-xs font-mono text-muted-foreground break-all bg-black/30 rounded p-2">
          {result!.apk_path}
        </p>
        {result!.warnings.length > 0 && (
          <div className="space-y-1">
            <p className="text-xs font-medium text-amber-400">Warnings:</p>
            {result!.warnings.map((w, i) => (
              <p key={i} className="text-xs text-amber-300/80">• {w}</p>
            ))}
          </div>
        )}
        <div className="flex gap-2">
          <Button size="sm" variant="outline" onClick={() => openFolder(result!.apk_path)}>
            <FolderOpen className="h-4 w-4" />
            Open Folder
          </Button>
          <Button size="sm" variant="ghost" onClick={() => copyPath(result!.apk_path)}>
            <Copy className="h-4 w-4" />
            Copy Path
          </Button>
          <Button size="sm" variant="ghost" onClick={onReset}>
            Build Another
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
