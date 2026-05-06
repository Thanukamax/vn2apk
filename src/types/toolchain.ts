export interface ToolStatus {
  name: string;
  binary: string;
  found: boolean;
  version: string | null;
  path: string | null;
  required: boolean;
}

export type ToolName = "java" | "adb" | "sdkmanager" | "gradle" | "node" | "cordova" | "apksigner";
