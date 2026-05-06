import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { FolderOpen } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { AppSettings } from "@/types/settings";

interface FieldDef {
  key: keyof AppSettings;
  label: string;
  placeholder: string;
  isDir?: boolean;
  isFile?: boolean;
}

const FIELDS: FieldDef[] = [
  {
    key: "android_sdk_path",
    label: "Android SDK Path",
    placeholder: "~/android-sdk",
    isDir: true,
  },
  {
    key: "gradle_home",
    label: "Gradle Home",
    placeholder: "~/gradle/gradle-8.12",
    isDir: true,
  },
  { key: "java_home", label: "Java Home", placeholder: "/usr/lib/jvm/java-17-openjdk", isDir: true },
  { key: "node_path", label: "Node.js Path (optional)", placeholder: "/usr/bin/node" },
  {
    key: "cordova_path",
    label: "Cordova Path (optional)",
    placeholder: "~/.npm-global/bin/cordova",
  },
  {
    key: "output_dir",
    label: "Output Directory",
    placeholder: "~/Downloads",
    isDir: true,
  },
  {
    key: "build_tools_version",
    label: "Build Tools Version",
    placeholder: "34.0.0",
  },
  {
    key: "renpy_sdk_path",
    label: "Ren'Py SDK Path (for Ren'Py games)",
    placeholder: "~/renpy-8.3.7",
    isDir: true,
  },
];

interface Props {
  settings: AppSettings;
  onChange: (settings: AppSettings) => void;
}

export function SettingsForm({ settings, onChange }: Props) {
  const update = (key: keyof AppSettings, value: string) =>
    onChange({ ...settings, [key]: value });

  const pickDir = async (key: keyof AppSettings) => {
    const selected = await openDialog({ directory: true, multiple: false });
    if (typeof selected === "string") update(key, selected);
  };

  return (
    <div className="space-y-4">
      {FIELDS.map(({ key, label, placeholder, isDir }) => (
        <div key={key} className="space-y-1.5">
          <Label htmlFor={key}>{label}</Label>
          <div className="flex gap-2">
            <Input
              id={key}
              value={settings[key] as string}
              onChange={(e) => update(key, e.target.value)}
              placeholder={placeholder}
              className="font-mono text-xs"
            />
            {isDir && (
              <Button
                type="button"
                variant="outline"
                size="icon"
                onClick={() => pickDir(key)}
                title="Browse"
              >
                <FolderOpen className="h-4 w-4" />
              </Button>
            )}
          </div>
        </div>
      ))}
    </div>
  );
}
