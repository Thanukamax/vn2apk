import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useState } from "react";
import { AppSettings, DEFAULT_SETTINGS } from "@/types/settings";

export function useSettings() {
  const [settings, setSettings] = useState<AppSettings>(DEFAULT_SETTINGS);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    invoke<AppSettings>("cmd_get_settings")
      .then(setSettings)
      .finally(() => setLoading(false));
  }, []);

  const saveSettings = useCallback(async (updated: AppSettings) => {
    setSaving(true);
    try {
      await invoke("cmd_save_settings", { settings: updated });
      setSettings(updated);
    } finally {
      setSaving(false);
    }
  }, []);

  return { settings, setSettings, saveSettings, loading, saving };
}
