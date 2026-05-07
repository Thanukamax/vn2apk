import { invoke } from "@tauri-apps/api/core";
import { Loader2, Save, Wand2 } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { KeystoreSection } from "@/components/settings/KeystoreSection";
import { SettingsForm } from "@/components/settings/SettingsForm";
import { useSettings } from "@/hooks/useSettings";
import { AppSettings } from "@/types/settings";

export function SettingsPage() {
  const { settings, setSettings, saveSettings, loading, saving } = useSettings();
  const [detecting, setDetecting] = useState(false);

  const handleSave = async () => {
    try {
      await saveSettings(settings);
      toast.success("Settings saved");
    } catch (e) {
      toast.error(`Failed to save: ${e}`);
    }
  };

  const handleAutoDetect = async () => {
    setDetecting(true);
    try {
      const detected = await invoke<AppSettings>("cmd_autodetect_settings");

      // Merge: only overwrite fields that were actually found (non-empty)
      const merged: AppSettings = { ...settings };
      const fieldKeys: (keyof AppSettings)[] = [
        "android_sdk_path",
        "gradle_home",
        "java_home",
        "node_path",
        "cordova_path",
        "build_tools_version",
        "output_dir",
        "renpy_sdk_path",
      ];

      const found: string[] = [];
      const missing: string[] = [];

      for (const key of fieldKeys) {
        const val = detected[key] as string;
        if (val && val.trim().length > 0) {
          (merged as unknown as Record<string, unknown>)[key] = val;
          found.push(key.replace(/_/g, " "));
        } else {
          missing.push(key.replace(/_/g, " "));
        }
      }

      setSettings(merged);

      if (found.length > 0) {
        toast.success(`Auto-detected: ${found.join(", ")}`, { duration: 5000 });
      }
      if (missing.length > 0) {
        toast.warning(`Not found (fill manually): ${missing.join(", ")}`, { duration: 6000 });
      }
    } catch (e) {
      toast.error(`Auto-detect failed: ${e}`);
    } finally {
      setDetecting(false);
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center h-full">
        <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-6 p-6 overflow-y-auto h-full max-w-2xl">
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-xl font-semibold">Settings</h1>
          <p className="text-sm text-muted-foreground mt-1">
            Configure SDK paths, keystore, and output directory.
          </p>
        </div>
        <Button
          variant="outline"
          onClick={handleAutoDetect}
          disabled={detecting}
          className="gap-2 shrink-0"
        >
          {detecting ? (
            <Loader2 className="h-4 w-4 animate-spin" />
          ) : (
            <Wand2 className="h-4 w-4" />
          )}
          Auto-detect Paths
        </Button>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="text-base">SDK & Tools</CardTitle>
        </CardHeader>
        <CardContent>
          <SettingsForm settings={settings} onChange={setSettings} />
        </CardContent>
      </Card>

      <Separator />

      <KeystoreSection settings={settings} onChange={setSettings} />

      <div className="flex justify-end pb-6">
        <Button onClick={handleSave} disabled={saving}>
          {saving ? <Loader2 className="h-4 w-4 animate-spin" /> : <Save className="h-4 w-4" />}
          Save Settings
        </Button>
      </div>
    </div>
  );
}
