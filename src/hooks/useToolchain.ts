import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";
import { ToolStatus } from "@/types/toolchain";

export interface InstallLog {
  level: string;
  message: string;
  timestamp: number;
}

export function useToolchain() {
  const [tools, setTools] = useState<ToolStatus[]>([]);
  const [loading, setLoading] = useState(false);
  const [installing, setInstalling] = useState<string | null>(null);
  const [installLogs, setInstallLogs] = useState<InstallLog[]>([]);
  const unlistenRef = useRef<(() => void) | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const result = await invoke<ToolStatus[]>("cmd_check_toolchain");
      setTools(result);
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  const installTool = useCallback(
    async (toolName: string) => {
      setInstalling(toolName);
      setInstallLogs([]);

      unlistenRef.current = await listen<InstallLog>("tool-install-log", (event) => {
        setInstallLogs((prev) => [...prev, event.payload]);
      });

      try {
        await invoke("cmd_install_tool", { toolName });
      } finally {
        unlistenRef.current?.();
        unlistenRef.current = null;
        setInstalling(null);
        await refresh();
      }
    },
    [refresh]
  );

  return { tools, loading, installing, installLogs, refresh, installTool };
}
