use crate::error::AppError;
use crate::pipeline::types::ValidationResult;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EngineKind {
    RpgMakerMV,
    RpgMakerMZ,
    TyranoBuilder,
    RenPy,
    NScripter,
    Kirikiri,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineDetectResult {
    pub engine: EngineKind,
    pub confidence: u8,
    pub details: String,
}

pub fn detect_engine(game_path: &str) -> Result<EngineDetectResult, AppError> {
    let base = Path::new(game_path);
    if !base.exists() {
        return Err(AppError::msg("Game folder does not exist"));
    }

    // RPGMZ
    if base.join("www/js/rmmz_core.js").exists() {
        return Ok(EngineDetectResult {
            engine: EngineKind::RpgMakerMZ,
            confidence: 95,
            details: "Found www/js/rmmz_core.js".into(),
        });
    }
    // RPGMV
    if base.join("www/js/rpg_core.js").exists() {
        return Ok(EngineDetectResult {
            engine: EngineKind::RpgMakerMV,
            confidence: 95,
            details: "Found www/js/rpg_core.js".into(),
        });
    }
    if base.join("Game.rpgproject").exists() {
        return Ok(EngineDetectResult {
            engine: EngineKind::RpgMakerMV,
            confidence: 90,
            details: "Found Game.rpgproject".into(),
        });
    }
    // TyranoBuilder — exports to HTML5 with a tyrano/ directory at root
    if base.join("tyrano/libs/tyrano.js").exists()
        || base.join("tyrano/libs/tyrano.min.js").exists()
        || (base.join("tyrano").is_dir() && base.join("index.html").exists())
    {
        return Ok(EngineDetectResult {
            engine: EngineKind::TyranoBuilder,
            confidence: 92,
            details: "Found tyrano/ directory with index.html".into(),
        });
    }
    // Ren'Py
    if base.join("renpy").is_dir()
        || std::fs::read_dir(base.join("game"))
            .map(|mut d| {
                d.any(|e| {
                    e.map(|e| {
                        e.path().extension().map(|x| x == "rpy").unwrap_or(false)
                    })
                    .unwrap_or(false)
                })
            })
            .unwrap_or(false)
    {
        return Ok(EngineDetectResult {
            engine: EngineKind::RenPy,
            confidence: 85,
            details: "Found Ren'Py markers (renpy/ dir or game/*.rpy)".into(),
        });
    }
    // NScripter
    if base.join("nscript.dat").exists() || base.join("arc.nsa").exists() {
        return Ok(EngineDetectResult {
            engine: EngineKind::NScripter,
            confidence: 90,
            details: "Found NScripter data files".into(),
        });
    }
    // Kirikiri
    if base.join("data.xp3").exists() || base.join("startup.tjs").exists() {
        return Ok(EngineDetectResult {
            engine: EngineKind::Kirikiri,
            confidence: 90,
            details: "Found KiriKiri data files".into(),
        });
    }

    Ok(EngineDetectResult {
        engine: EngineKind::Unknown,
        confidence: 0,
        details: "No known engine markers found".into(),
    })
}

pub fn validate_game_folder(game_path: &str) -> Result<ValidationResult, AppError> {
    let base = Path::new(game_path);
    let mut issues = Vec::new();

    if !base.exists() {
        return Ok(ValidationResult {
            valid: false,
            issues: vec!["Folder does not exist".into()],
        });
    }

    // Detect engine first so we can validate appropriately
    let engine = detect_engine(game_path).map(|r| r.engine).unwrap_or(EngineKind::Unknown);

    match engine {
        EngineKind::TyranoBuilder => {
            if !base.join("tyrano").is_dir() {
                issues.push("Missing tyrano/ directory".into());
            }
            if !base.join("index.html").exists() {
                issues.push("Missing index.html".into());
            }
        }
        EngineKind::RenPy => {
            if !base.join("game").is_dir() {
                issues.push("Missing game/ directory".into());
            }
            let has_rpy = std::fs::read_dir(base.join("game"))
                .map(|mut d| {
                    d.any(|e| {
                        e.map(|e| e.path().extension().map(|x| x == "rpy").unwrap_or(false))
                            .unwrap_or(false)
                    })
                })
                .unwrap_or(false);
            if !has_rpy {
                issues.push("No .rpy scripts found in game/".into());
            }
        }
        _ => {
            // RPGMV / MZ / Unknown — require www/
            if !base.join("www").is_dir() {
                issues.push("Missing www/ directory (expected for RPGMV/MZ)".into());
            }
            if !base.join("www/index.html").exists() && !base.join("index.html").exists() {
                issues.push("Missing index.html".into());
            }
        }
    }

    Ok(ValidationResult {
        valid: issues.is_empty(),
        issues,
    })
}
