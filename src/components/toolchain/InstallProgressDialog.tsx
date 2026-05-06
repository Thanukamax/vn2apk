import { useEffect, useRef } from "react";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { ScrollArea } from "@/components/ui/scroll-area";
import { InstallLog } from "@/hooks/useToolchain";
import { cn } from "@/lib/utils";

interface Props {
  open: boolean;
  toolName: string;
  logs: InstallLog[];
}

export function InstallProgressDialog({ open, toolName, logs }: Props) {
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs.length]);

  return (
    <Dialog open={open}>
      <DialogContent className="max-w-2xl" onInteractOutside={(e) => e.preventDefault()}>
        <DialogHeader>
          <DialogTitle>Installing {toolName}…</DialogTitle>
        </DialogHeader>
        <ScrollArea className="h-72 rounded-md border border-border bg-black/60 p-3">
          <div className="font-mono text-xs space-y-0.5">
            {logs.length === 0 ? (
              <span className="text-muted-foreground">Waiting for output…</span>
            ) : (
              logs.map((l, i) => (
                <div
                  key={i}
                  className={cn(
                    l.level === "error" ? "text-red-400" : l.level === "warn" ? "text-amber-400" : "text-gray-300"
                  )}
                >
                  {l.message}
                </div>
              ))
            )}
            <div ref={bottomRef} />
          </div>
        </ScrollArea>
      </DialogContent>
    </Dialog>
  );
}
