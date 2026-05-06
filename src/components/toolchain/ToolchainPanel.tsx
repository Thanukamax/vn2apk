import { RefreshCw } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useToolchain } from "@/hooks/useToolchain";
import { InstallProgressDialog } from "./InstallProgressDialog";
import { ToolchainItem } from "./ToolchainItem";

export function ToolchainPanel() {
  const { tools, loading, installing, installLogs, refresh, installTool } = useToolchain();

  const allOk = tools.every((t) => !t.required || t.found);
  const missing = tools.filter((t) => t.required && !t.found).length;

  return (
    <div className="space-y-4">
      <Card>
        <CardHeader className="pb-3">
          <div className="flex items-center justify-between">
            <CardTitle className="text-base">Build Tools</CardTitle>
            <div className="flex items-center gap-2">
              <Badge variant={allOk ? "success" : "warning"}>
                {allOk ? "All ready" : `${missing} missing`}
              </Badge>
              <Button
                variant="ghost"
                size="icon"
                className="h-8 w-8"
                onClick={refresh}
                disabled={loading}
              >
                <RefreshCw className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} />
              </Button>
            </div>
          </div>
        </CardHeader>
        <CardContent className="space-y-2">
          {loading && tools.length === 0 ? (
            <p className="text-sm text-muted-foreground">Detecting tools…</p>
          ) : (
            tools.map((tool) => (
              <ToolchainItem
                key={tool.binary}
                tool={tool}
                installing={installing === tool.name}
                onInstall={installTool}
              />
            ))
          )}
        </CardContent>
      </Card>

      <InstallProgressDialog
        open={installing !== null}
        toolName={installing ?? ""}
        logs={installLogs}
      />
    </div>
  );
}
