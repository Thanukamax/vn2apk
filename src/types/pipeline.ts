export type EngineKind =
  | "RpgMakerMV"
  | "RpgMakerMZ"
  | "TyranoBuilder"
  | "RenPy"
  | "NScripter"
  | "Kirikiri"
  | "Unknown";

export interface EngineDetectResult {
  engine: EngineKind;
  confidence: number;
  details: string;
}

export interface ValidationResult {
  valid: boolean;
  issues: string[];
}

export interface BuildOptions {
  app_id: string;
  app_name: string;
  version_name: string;
  version_code: number;
  icon_path: string | null;
  storepass: string;
  keypass: string;
  engine: string;
}

export interface BuildResult {
  apk_path: string;
  warnings: string[];
}

export type PipelineStage =
  | "Validate"
  | "Scaffold"
  | "CopyAssets"
  | "PatchConfig"
  | "AddPlatform"
  | "Build"
  | "Align"
  | "Sign"
  | "Output";

export type StageStatus = "Pending" | "Running" | "Done" | "Failed";

export interface PipelineStageEvent {
  stage: PipelineStage;
  status: StageStatus;
}

export interface LogLine {
  level: "info" | "warn" | "error";
  message: string;
  timestamp: number;
}
