import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { FolderOpen } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { BuildOptions } from "@/types/pipeline";

interface Props {
  options: Partial<BuildOptions>;
  onChange: (opts: Partial<BuildOptions>) => void;
  disabled?: boolean;
}

export function BuildOptionsForm({ options, onChange, disabled }: Props) {
  const set = (key: keyof BuildOptions, val: string | number) =>
    onChange({ ...options, [key]: val });

  const pickIcon = async () => {
    const f = await openDialog({
      multiple: false,
      filters: [{ name: "Image", extensions: ["png"] }],
    });
    if (typeof f === "string") set("icon_path", f);
  };

  return (
    <div className="grid grid-cols-2 gap-3">
      <div className="col-span-2 space-y-1">
        <Label>App ID</Label>
        <Input
          value={options.app_id ?? ""}
          onChange={(e) => set("app_id", e.target.value)}
          placeholder="com.example.mygame"
          className="font-mono text-xs"
          disabled={disabled}
        />
      </div>
      <div className="space-y-1">
        <Label>App Name</Label>
        <Input
          value={options.app_name ?? ""}
          onChange={(e) => set("app_name", e.target.value)}
          placeholder="My Game"
          disabled={disabled}
        />
      </div>
      <div className="space-y-1">
        <Label>Version</Label>
        <Input
          value={options.version_name ?? "1.0.0"}
          onChange={(e) => set("version_name", e.target.value)}
          placeholder="1.0.0"
          disabled={disabled}
        />
      </div>
      <div className="space-y-1">
        <Label>Version Code</Label>
        <Input
          type="number"
          value={options.version_code ?? 1}
          onChange={(e) => set("version_code", parseInt(e.target.value) || 1)}
          min={1}
          disabled={disabled}
        />
      </div>
      <div className="space-y-1">
        <Label>Icon (PNG, optional)</Label>
        <div className="flex gap-2">
          <Input
            value={options.icon_path ?? ""}
            onChange={(e) => set("icon_path", e.target.value)}
            placeholder="auto-detect"
            className="text-xs font-mono"
            disabled={disabled}
          />
          <Button type="button" variant="outline" size="icon" onClick={pickIcon} disabled={disabled}>
            <FolderOpen className="h-4 w-4" />
          </Button>
        </div>
      </div>
    </div>
  );
}
