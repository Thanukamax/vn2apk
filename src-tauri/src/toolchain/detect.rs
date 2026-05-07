use super::types::ToolStatus;
use crate::settings::AppSettings;
use std::process::Command;

fn run_version(binary: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(binary).args(args).output().ok()?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let stderr = String::from_utf8_lossy(&out.stderr);
    let combined = if stdout.trim().is_empty() { stderr } else { stdout };
    combined.lines().next().map(|l| l.trim().to_string())
}

fn detect_tool(name: &str, binary: &str, version_args: &[&str], required: bool) -> ToolStatus {
    match which::which(binary) {
        Ok(path) => {
            let version = run_version(binary, version_args);
            ToolStatus {
                name: name.to_string(),
                binary: binary.to_string(),
                found: true,
                version,
                path: Some(path.to_string_lossy().to_string()),
                required,
            }
        }
        Err(_) => ToolStatus {
            name: name.to_string(),
            binary: binary.to_string(),
            found: false,
            version: None,
            path: None,
            required,
        },
    }
}

/// Try a binary at an explicit path first, fall back to PATH.
fn detect_tool_with_hint(
    name: &str,
    binary: &str,
    hint_path: Option<&str>,
    version_args: &[&str],
    required: bool,
) -> ToolStatus {
    if let Some(hint) = hint_path.filter(|p| !p.is_empty()) {
        let candidate = std::path::Path::new(hint).join(binary);
        if candidate.exists() {
            let version = run_version(candidate.to_str().unwrap_or(binary), version_args);
            return ToolStatus {
                name: name.to_string(),
                binary: binary.to_string(),
                found: true,
                version,
                path: Some(candidate.to_string_lossy().to_string()),
                required,
            };
        }
    }
    detect_tool(name, binary, version_args, required)
}

/// Detect a tool when settings stores the full binary path (not a bin directory).
fn detect_tool_at_path(
    name: &str,
    binary: &str,
    full_path: &str,
    version_args: &[&str],
    required: bool,
) -> ToolStatus {
    if !full_path.is_empty() {
        let p = std::path::Path::new(full_path);
        if p.exists() {
            let version = run_version(full_path, version_args);
            return ToolStatus {
                name: name.to_string(),
                binary: binary.to_string(),
                found: true,
                version,
                path: Some(full_path.to_string()),
                required,
            };
        }
    }
    detect_tool(name, binary, version_args, required)
}

fn detect_renpy_sdk(sdk_path: &str) -> ToolStatus {
    let name = "Ren'Py SDK";
    let binary = "renpy.sh";

    if !sdk_path.trim().is_empty() {
        let sh = std::path::Path::new(sdk_path).join("renpy.sh");
        let version = std::path::Path::new(sdk_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .and_then(|dir_name| {
                let s = dir_name.strip_prefix("renpy-").unwrap_or(&dir_name).to_string();
                let s = s.strip_suffix("-sdk").unwrap_or(&s).to_string();
                if s.contains('.') { Some(s) } else { None }
            });
        return ToolStatus {
            name: name.to_string(),
            binary: binary.to_string(),
            found: sh.exists(),
            version,
            path: if sh.exists() { Some(sdk_path.to_string()) } else { None },
            required: false,
        };
    }

    // Fallback: check PATH for renpy.sh
    match which::which(binary) {
        Ok(path) => ToolStatus {
            name: name.to_string(),
            binary: binary.to_string(),
            found: true,
            version: None,
            path: Some(path.to_string_lossy().to_string()),
            required: false,
        },
        Err(_) => ToolStatus {
            name: name.to_string(),
            binary: binary.to_string(),
            found: false,
            version: None,
            path: None,
            required: false,
        },
    }
}

fn detect_rapt(sdk_path: &str) -> ToolStatus {
    let name = "RAPT";
    let binary = "rapt";

    if sdk_path.is_empty() {
        return ToolStatus {
            name: name.to_string(),
            binary: binary.to_string(),
            found: false,
            version: None,
            path: None,
            required: false,
        };
    }

    let gradle_file = std::path::Path::new(sdk_path).join("rapt/project/build.gradle");
    if gradle_file.exists() {
        ToolStatus {
            name: name.to_string(),
            binary: binary.to_string(),
            found: true,
            version: Some("installed".to_string()),
            path: Some(
                std::path::Path::new(sdk_path)
                    .join("rapt")
                    .to_string_lossy()
                    .to_string(),
            ),
            required: false,
        }
    } else {
        ToolStatus {
            name: name.to_string(),
            binary: binary.to_string(),
            found: false,
            version: None,
            path: None,
            required: false,
        }
    }
}

pub fn check_all_tools(settings: &AppSettings) -> Vec<ToolStatus> {
    let sdk = settings.android_sdk_path.trim_end_matches('/');
    let build_tools_bin = if sdk.is_empty() {
        None
    } else {
        Some(format!("{sdk}/build-tools/{}", settings.build_tools_version))
    };
    let platform_tools_bin = if sdk.is_empty() {
        None
    } else {
        Some(format!("{sdk}/platform-tools"))
    };
    let cmdline_bin = if sdk.is_empty() {
        None
    } else {
        Some(format!("{sdk}/cmdline-tools/latest/bin"))
    };
    let gradle_bin = if settings.gradle_home.is_empty() {
        None
    } else {
        Some(format!("{}/bin", settings.gradle_home))
    };
    let java_bin = if settings.java_home.is_empty() {
        None
    } else {
        Some(format!("{}/bin", settings.java_home))
    };

    vec![
        detect_tool_with_hint("Java", "java", java_bin.as_deref(), &["-version"], true),
        detect_tool_with_hint(
            "ADB",
            "adb",
            platform_tools_bin.as_deref(),
            &["--version"],
            true,
        ),
        detect_tool_with_hint(
            "sdkmanager",
            "sdkmanager",
            cmdline_bin.as_deref(),
            &["--version"],
            true,
        ),
        detect_tool_with_hint(
            "Gradle",
            "gradle",
            gradle_bin.as_deref(),
            &["--version"],
            true,
        ),
        detect_tool_at_path("Node.js", "node", &settings.node_path, &["--version"], true),
        detect_tool_at_path("Cordova", "cordova", &settings.cordova_path, &["--version"], true),
        detect_tool_with_hint(
            "apksigner",
            "apksigner",
            build_tools_bin.as_deref(),
            &["--version"],
            true,
        ),
        detect_renpy_sdk(&settings.renpy_sdk_path),
        detect_rapt(&settings.renpy_sdk_path),
    ]
}
