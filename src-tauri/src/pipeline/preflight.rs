use crate::settings::AppSettings;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightIssue {
    pub severity: String, // "error" | "warning"
    pub message: String,
    pub hint: String,
}

impl PreflightIssue {
    fn error(message: impl Into<String>, hint: impl Into<String>) -> Self {
        Self {
            severity: "error".into(),
            message: message.into(),
            hint: hint.into(),
        }
    }
    fn warning(message: impl Into<String>, hint: impl Into<String>) -> Self {
        Self {
            severity: "warning".into(),
            message: message.into(),
            hint: hint.into(),
        }
    }
}

/// Returns available disk space in /tmp in megabytes, using `df`.
fn tmp_free_mb() -> Option<u64> {
    let out = Command::new("df")
        .args(["--output=avail", "-B1", "/tmp"])
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    // Output looks like:
    //    Avail
    // 12345678
    for line in text.lines().skip(1) {
        if let Ok(bytes) = line.trim().parse::<u64>() {
            return Some(bytes / (1024 * 1024));
        }
    }
    None
}

/// Check whether `java` is available (via JAVA_HOME or PATH).
fn java_available(settings: &AppSettings) -> bool {
    // Try JAVA_HOME/bin/java first
    if !settings.java_home.is_empty() {
        let java = Path::new(&settings.java_home).join("bin/java");
        if java.exists() {
            return true;
        }
    }
    // Try PATH
    Command::new("java")
        .arg("-version")
        .output()
        .map(|o| o.status.success() || !o.stderr.is_empty()) // `java -version` writes to stderr
        .unwrap_or(false)
}

pub fn run_preflight(engine: &str, game_path: &str, settings: &AppSettings) -> Vec<PreflightIssue> {
    let mut issues: Vec<PreflightIssue> = Vec::new();

    // ── Always checks ─────────────────────────────────────────────────────────

    // 1. game_path exists and is a directory
    if game_path.is_empty() {
        issues.push(PreflightIssue::error(
            "No game folder selected.",
            "Choose a game folder before building.",
        ));
    } else {
        let gp = Path::new(game_path);
        if !gp.exists() {
            issues.push(PreflightIssue::error(
                format!("Game folder does not exist: {game_path}"),
                "Select a valid game folder.",
            ));
        } else if !gp.is_dir() {
            issues.push(PreflightIssue::error(
                format!("Game path is not a directory: {game_path}"),
                "Select a folder, not a file.",
            ));
        }
    }

    // 2. keystore_path is non-empty
    if settings.keystore_path.is_empty() {
        issues.push(PreflightIssue::error(
            "Keystore path is not set.",
            "Go to Settings → Signing and generate or select a keystore.",
        ));
    }

    // 3. Java available
    if !java_available(settings) {
        issues.push(PreflightIssue::error(
            "Java (JDK) not found.",
            "Set JAVA_HOME in Settings or install JDK 17+ via the Toolchain page.",
        ));
    }

    // 4. Android SDK path set and build-tools/ exists
    if settings.android_sdk_path.is_empty() {
        issues.push(PreflightIssue::error(
            "Android SDK path is not configured.",
            "Set it in Settings → Android SDK, or install it via the Toolchain page.",
        ));
    } else {
        let bt = Path::new(&settings.android_sdk_path).join("build-tools");
        if !bt.is_dir() {
            issues.push(PreflightIssue::error(
                "Android SDK build-tools/ directory not found.",
                "Run sdkmanager to install build-tools, or re-install via the Toolchain page.",
            ));
        }
    }

    // 5. Disk space in /tmp > 500 MB
    match tmp_free_mb() {
        Some(mb) if mb < 500 => {
            issues.push(PreflightIssue::error(
                format!("Low disk space in /tmp: {mb} MB free (need > 500 MB)."),
                "Free up space in /tmp and try again.",
            ));
        }
        None => {
            issues.push(PreflightIssue::warning(
                "Could not determine free disk space in /tmp.",
                "Ensure /tmp has at least 500 MB free before building.",
            ));
        }
        _ => {}
    }

    // ── Engine-specific checks ─────────────────────────────────────────────────

    match engine {
        "RenPy" => {
            // renpy_sdk_path is set
            if settings.renpy_sdk_path.is_empty() {
                issues.push(PreflightIssue::error(
                    "Ren'Py SDK path is not set.",
                    "Set it in Settings → Ren'Py SDK.",
                ));
            } else {
                let sdk = Path::new(&settings.renpy_sdk_path);

                // renpy.sh exists
                if !sdk.join("renpy.sh").exists() {
                    issues.push(PreflightIssue::error(
                        "renpy.sh not found in the configured SDK path.",
                        "Check that the Ren'Py SDK path is correct.",
                    ));
                }

                // RAPT installed
                if !sdk.join("rapt/project/build.gradle").exists() {
                    issues.push(PreflightIssue::error(
                        "RAPT is not installed.",
                        "Open the Ren'Py Launcher → Android → Install SDK & Install RAPT.",
                    ));
                }
            }

            // android.json sanity check (warning only, we can generate one)
            if !game_path.is_empty() {
                let android_json = Path::new(game_path).join("android.json");
                if android_json.exists() {
                    match std::fs::read_to_string(&android_json) {
                        Ok(text) => match serde_json::from_str::<serde_json::Value>(&text) {
                            Ok(val) => {
                                let has_package = val.get("package").and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false);
                                let has_name = val.get("name").and_then(|v| v.as_str()).map(|s| !s.is_empty()).unwrap_or(false);
                                if !has_package || !has_name {
                                    issues.push(PreflightIssue::warning(
                                        "android.json is missing 'package' or 'name' keys.",
                                        "Edit android.json to include valid 'package' and 'name' values, or delete it to let vn2apk regenerate it.",
                                    ));
                                }
                            }
                            Err(_) => {
                                issues.push(PreflightIssue::warning(
                                    "android.json exists but is not valid JSON.",
                                    "Fix or delete android.json — vn2apk will regenerate a valid one.",
                                ));
                            }
                        },
                        Err(_) => {} // unreadable — pipeline will handle it
                    }
                }
            }
        }

        "RPGMV" | "RPGMZ" => {
            // node available
            let node_ok = if !settings.node_path.is_empty() {
                Path::new(&settings.node_path).exists()
            } else {
                std::process::Command::new("node")
                    .arg("--version")
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
            };
            if !node_ok {
                issues.push(PreflightIssue::error(
                    "Node.js not found.",
                    "Set Node.js path in Settings or install it via the Toolchain page.",
                ));
            }

            // cordova available
            let cordova_ok = if !settings.cordova_path.is_empty() {
                Path::new(&settings.cordova_path).exists()
            } else {
                std::process::Command::new("cordova")
                    .arg("--version")
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
            };
            if !cordova_ok {
                issues.push(PreflightIssue::error(
                    "Cordova not found.",
                    "Set Cordova path in Settings or install it via the Toolchain page.",
                ));
            }
        }

        _ => {}
    }

    issues
}
