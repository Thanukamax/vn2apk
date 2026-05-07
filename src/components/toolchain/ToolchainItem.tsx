import {
  Binary,
  Box,
  Coffee,
  Download,
  Gamepad2,
  Loader2,
  Smartphone,
  Terminal,
  Wrench,
  Zap,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ToolStatus } from "@/types/toolchain";

const TOOL_ICONS: Record<string, React.ComponentType<{ className?: string }>> = {
  Java: Coffee,
  ADB: Smartphone,
  sdkmanager: Box,
  Gradle: Zap,
  "Node.js": Terminal,
  Cordova: Binary,
  apksigner: Wrench,
  "Ren'Py SDK": Gamepad2,
  RAPT: Gamepad2,
};

const INSTALL_HINTS: Record<string, string> = {
  Java: "jdk",
  sdkmanager: "android-cmdline-tools",
  Gradle: "gradle",
  "Node.js": "node",
  Cordova: "cordova",
  "Ren'Py SDK": "renpy-sdk",
};

// Tools that must be installed manually — show an informational note instead of an install button.
const MANUAL_INSTALL_TOOLS = new Set(["RAPT"]);

interface Props {
  tool: ToolStatus;
  installing: boolean;
  onInstall: (toolName: string) => void;
}

export function ToolchainItem({ tool, installing, onInstall }: Props) {
  const Icon = TOOL_ICONS[tool.name] ?? Wrench;
  const installKey = INSTALL_HINTS[tool.name];
  const isManual = MANUAL_INSTALL_TOOLS.has(tool.name);

  return (
    <div className="flex items-center gap-4 rounded-lg border border-border bg-card px-4 py-3">
      <Icon className="h-5 w-5 text-muted-foreground shrink-0" />
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="text-sm font-medium">{tool.name}</span>
          {tool.required && (
            <span className="text-[10px] text-muted-foreground uppercase tracking-wide">
              required
            </span>
          )}
        </div>
        {tool.found && tool.version ? (
          <p className="text-xs text-muted-foreground font-mono truncate mt-0.5">{tool.version}</p>
        ) : tool.found ? (
          <p className="text-xs text-muted-foreground mt-0.5">found (version unknown)</p>
        ) : (
          <p className="text-xs text-destructive-foreground mt-0.5">Not found on PATH</p>
        )}
      </div>
      <div className="flex items-center gap-3 shrink-0">
        <Badge variant={tool.found ? "success" : "destructive"}>
          {tool.found ? "OK" : "Missing"}
        </Badge>
        {!tool.found && isManual && (
          <span className="text-xs text-muted-foreground italic">
            Manual install required
          </span>
        )}
        {!tool.found && !isManual && installKey && (
          <Button
            size="sm"
            variant="outline"
            disabled={installing}
            onClick={() => onInstall(installKey)}
            className="h-7 text-xs"
          >
            {installing ? (
              <Loader2 className="h-3 w-3 animate-spin" />
            ) : (
              <>
                <Download className="h-3 w-3" />
                Install
              </>
            )}
          </Button>
        )}
      </div>
    </div>
  );
}
