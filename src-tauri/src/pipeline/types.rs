use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildOptions {
    pub app_id: String,
    pub app_name: String,
    pub version_name: String,
    pub version_code: u32,
    pub icon_path: Option<String>,
    pub storepass: String,
    pub keypass: String,
    pub engine: String, // "RpgMakerMV" | "RpgMakerMZ" | "TyranoBuilder" | "RenPy"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildResult {
    pub apk_path: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PipelineStage {
    Validate,
    Scaffold,
    CopyAssets,
    PatchConfig,
    AddPlatform,
    Build,
    Align,
    Sign,
    Output,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StageStatus {
    Pending,
    Running,
    Done,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineStageEvent {
    pub stage: PipelineStage,
    pub status: StageStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogLine {
    pub level: String,
    pub message: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub issues: Vec<String>,
}
