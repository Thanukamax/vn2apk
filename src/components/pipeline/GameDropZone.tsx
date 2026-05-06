import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { FolderOpen, FolderSearch } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";

interface Props {
  onFolderSelected: (path: string) => void;
  disabled?: boolean;
}

export function GameDropZone({ onFolderSelected, disabled }: Props) {
  const [dragging, setDragging] = useState(false);
  const zoneRef = useRef<HTMLDivElement>(null);

  // Tauri v2 drag-drop: gives real filesystem paths via onDragDropEvent
  useEffect(() => {
    if (disabled) return;

    let unlisten: (() => void) | undefined;

    getCurrentWebview()
      .onDragDropEvent((event) => {
        const { type } = event.payload;

        if (type === "enter" || type === "over") {
          setDragging(true);
          return;
        }

        if (type === "leave" || type === "cancelled") {
          setDragging(false);
          return;
        }

        if (type === "drop") {
          setDragging(false);
          const paths: string[] =
            (event.payload as { type: string; paths?: string[] }).paths ?? [];
          // Pick the first path — if it looks like a folder use it directly,
          // otherwise take its parent directory.
          if (paths.length > 0) {
            const p = paths[0];
            // Heuristic: no extension = likely a directory
            const lastSeg = p.split("/").pop() ?? "";
            const isDir = !lastSeg.includes(".");
            onFolderSelected(isDir ? p : p.substring(0, p.lastIndexOf("/")));
          }
        }
      })
      .then((fn) => { unlisten = fn; })
      .catch(() => {}); // not fatal if unsupported

    return () => { unlisten?.(); };
  }, [disabled, onFolderSelected]);

  // Visual-only drag state from native browser events (fires when pointer is over this element)
  const onDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setDragging(true);
  }, []);
  const onDragLeave = useCallback(() => setDragging(false), []);
  // Suppress browser default drop so only the Tauri event fires
  const onDrop = useCallback((e: React.DragEvent) => { e.preventDefault(); }, []);

  const browse = useCallback(async () => {
    const selected = await openDialog({ directory: true, multiple: false });
    if (typeof selected === "string") onFolderSelected(selected);
  }, [onFolderSelected]);

  return (
    <div
      ref={zoneRef}
      className={cn(
        "relative flex flex-col items-center justify-center rounded-xl border-2 border-dashed transition-colors p-10 gap-4 select-none",
        dragging
          ? "border-primary bg-primary/10 scale-[1.01]"
          : "border-border bg-card/50 hover:border-primary/50 hover:bg-card",
        disabled && "opacity-50 pointer-events-none"
      )}
      onDragOver={onDragOver}
      onDragLeave={onDragLeave}
      onDrop={onDrop}
    >
      <div
        className={cn(
          "flex h-16 w-16 items-center justify-center rounded-full transition-colors",
          dragging ? "bg-primary/20" : "bg-muted"
        )}
      >
        <FolderSearch
          className={cn(
            "h-8 w-8 transition-colors",
            dragging ? "text-primary" : "text-muted-foreground"
          )}
        />
      </div>
      <div className="text-center space-y-1">
        <p className={cn("text-sm font-medium", dragging && "text-primary")}>
          {dragging ? "Release to select folder" : "Drop your game folder here"}
        </p>
        <p className="text-xs text-muted-foreground">
          RPGMV / RPGMaker MZ — needs a <code className="text-xs">www/</code> directory
        </p>
      </div>
      <Button variant="outline" size="sm" onClick={browse} disabled={disabled}>
        <FolderOpen className="h-4 w-4" />
        Browse for folder
      </Button>
    </div>
  );
}
