import { invoke } from "@tauri-apps/api/core";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { FolderOpen, KeyRound, Loader2 } from "lucide-react";
import { useState } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { AppSettings } from "@/types/settings";

interface Props {
  settings: AppSettings;
  onChange: (settings: AppSettings) => void;
}

export function KeystoreSection({ settings, onChange }: Props) {
  const [open_, setOpen_] = useState(false);
  const [generating, setGenerating] = useState(false);
  const [form, setForm] = useState({
    storepass: "",
    keypass: "",
    cn: "Unknown",
    ou: "Unknown",
    o: "Unknown",
    l: "Unknown",
    st: "Unknown",
    c: "US",
  });

  const pickKeystore = async () => {
    const selected = await openDialog({ multiple: false, filters: [{ name: "Keystore", extensions: ["jks", "keystore"] }] });
    if (typeof selected === "string") onChange({ ...settings, keystore_path: selected });
  };

  const saveKeystore = async () => {
    const selected = await saveDialog({ filters: [{ name: "Keystore", extensions: ["jks"] }] });
    if (typeof selected === "string") onChange({ ...settings, keystore_path: selected });
  };

  const generate = async () => {
    if (!settings.keystore_path) {
      await saveKeystore();
      return;
    }
    const dname = `CN=${form.cn}, OU=${form.ou}, O=${form.o}, L=${form.l}, ST=${form.st}, C=${form.c}`;
    setGenerating(true);
    try {
      await invoke("cmd_generate_keystore", {
        keystorePath: settings.keystore_path,
        alias: settings.keystore_alias,
        storepass: form.storepass,
        keypass: form.keypass,
        dname,
      });
      toast.success("Keystore generated successfully");
      setOpen_(false);
    } catch (e) {
      toast.error(`Failed: ${e}`);
    } finally {
      setGenerating(false);
    }
  };

  return (
    <div className="space-y-4 rounded-lg border border-border p-4">
      <div className="flex items-center gap-2">
        <KeyRound className="h-4 w-4 text-muted-foreground" />
        <span className="text-sm font-medium">Signing Keystore</span>
      </div>

      <div className="space-y-1.5">
        <Label htmlFor="keystore_path">Keystore File</Label>
        <div className="flex gap-2">
          <Input
            id="keystore_path"
            value={settings.keystore_path}
            onChange={(e) => onChange({ ...settings, keystore_path: e.target.value })}
            placeholder="~/my-release.jks"
            className="font-mono text-xs"
          />
          <Button type="button" variant="outline" size="icon" onClick={pickKeystore} title="Browse">
            <FolderOpen className="h-4 w-4" />
          </Button>
        </div>
      </div>

      <div className="space-y-1.5">
        <Label htmlFor="keystore_alias">Key Alias</Label>
        <Input
          id="keystore_alias"
          value={settings.keystore_alias}
          onChange={(e) => onChange({ ...settings, keystore_alias: e.target.value })}
          placeholder="key0"
        />
      </div>

      <Dialog open={open_} onOpenChange={setOpen_}>
        <DialogTrigger asChild>
          <Button variant="outline" size="sm">
            <KeyRound className="h-4 w-4" />
            Generate New Keystore
          </Button>
        </DialogTrigger>
        <DialogContent className="max-w-md">
          <DialogHeader>
            <DialogTitle>Generate Keystore</DialogTitle>
          </DialogHeader>
          <div className="space-y-3 text-sm">
            <p className="text-muted-foreground">
              Keystore will be saved to: <code className="text-xs">{settings.keystore_path || "(not set)"}</code>
            </p>
            {(
              [
                ["storepass", "Store Password"],
                ["keypass", "Key Password"],
                ["cn", "Common Name (CN)"],
                ["ou", "Org Unit (OU)"],
                ["o", "Organization (O)"],
                ["l", "City (L)"],
                ["st", "State (ST)"],
                ["c", "Country (C)"],
              ] as [keyof typeof form, string][]
            ).map(([k, label]) => (
              <div key={k} className="space-y-1">
                <Label>{label}</Label>
                <Input
                  type={k.includes("pass") ? "password" : "text"}
                  value={form[k]}
                  onChange={(e) => setForm((f) => ({ ...f, [k]: e.target.value }))}
                />
              </div>
            ))}
          </div>
          <DialogFooter>
            <Button
              onClick={generate}
              disabled={generating || !form.storepass || !form.keypass}
            >
              {generating ? <Loader2 className="h-4 w-4 animate-spin" /> : "Generate"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
