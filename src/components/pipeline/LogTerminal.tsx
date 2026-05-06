import { Trash2 } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { ScrollArea } from "@/components/ui/scroll-area";
import { LogLine } from "@/types/pipeline";
import { cn } from "@/lib/utils";

interface Props {
  logs: LogLine[];
  onClear?: () => void;
}

export function LogTerminal({ logs, onClear }: Props) {
  const bottomRef = useRef<HTMLDivElement>(null);
  const [autoScroll, setAutoScroll] = useState(true);

  useEffect(() => {
    if (autoScroll) {
      bottomRef.current?.scrollIntoView({ behavior: "auto" });
    }
  }, [logs.length, autoScroll]);

  return (
    <div className="flex flex-col rounded-lg border border-border overflow-hidden">
      <div className="flex items-center justify-between px-3 py-1.5 border-b border-border bg-black/40">
        <span className="text-xs text-muted-foreground font-mono">build output</span>
        <div className="flex items-center gap-2">
          <label className="flex items-center gap-1 text-xs text-muted-foreground cursor-pointer">
            <input
              type="checkbox"
              checked={autoScroll}
              onChange={(e) => setAutoScroll(e.target.checked)}
              className="w-3 h-3"
            />
            auto-scroll
          </label>
          {onClear && (
            <Button variant="ghost" size="icon" className="h-6 w-6" onClick={onClear}>
              <Trash2 className="h-3 w-3" />
            </Button>
          )}
        </div>
      </div>
      <ScrollArea className="h-64 bg-black/60">
        <div className="p-3 font-mono text-xs space-y-0.5 min-h-full">
          {logs.length === 0 ? (
            <span className="text-muted-foreground/50">Waiting for output…</span>
          ) : (
            logs.map((l, i) => (
              <div
                key={i}
                className={cn(
                  "leading-5 break-all",
                  l.level === "error" ? "text-red-400" :
                  l.level === "warn"  ? "text-amber-300" :
                                        "text-gray-300"
                )}
              >
                {l.message}
              </div>
            ))
          )}
          <div ref={bottomRef} />
        </div>
      </ScrollArea>
    </div>
  );
}
