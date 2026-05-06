use crate::settings::AppSettings;
use std::path::{Path, PathBuf};

/// Detect as many settings as possible from the environment.
/// Returns a partial AppSettings — only fields that were confidently found are set.
pub fn autodetect() -> AppSettings {
    let mut s = AppSettings::default();

    s.android_sdk_path = find_android_sdk();
    s.gradle_home = find_gradle_home();
    s.java_home = find_java_home();
    s.node_path = find_binary("node");
    s.cordova_path = find_cordova();
    s.build_tools_version = find_build_tools_version(&s.android_sdk_path);
    s.output_dir = default_output_dir();
    s.renpy_sdk_path = find_renpy_sdk();

    s
}

fn find_android_sdk() -> String {
    // 1. Env vars
    for var in &["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Ok(v) = std::env::var(var) {
            if !v.is_empty() && Path::new(&v).is_dir() {
                return v;
            }
        }
    }
    // 2. Common locations
    let home = home_dir();
    let candidates = [
        format!("{home}/Android/Sdk"),
        format!("{home}/android-sdk"),
        format!("{home}/.android/sdk"),
        "/opt/android-sdk".into(),
    ];
    candidates
        .iter()
        .find(|p| Path::new(p.as_str()).is_dir())
        .cloned()
        .unwrap_or_default()
}

fn find_gradle_home() -> String {
    if let Ok(v) = std::env::var("GRADLE_HOME") {
        if !v.is_empty() && Path::new(&v).is_dir() {
            return v;
        }
    }
    // Scan ~/gradle/ for highest version
    let home = home_dir();
    let dir = PathBuf::from(&home).join("gradle");
    if let Ok(entries) = std::fs::read_dir(&dir) {
        let mut versions: Vec<String> = entries
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                if e.path().is_dir() && name.starts_with("gradle-") {
                    Some(name)
                } else {
                    None
                }
            })
            .collect();
        // Sort descending so highest version comes first
        versions.sort_by(|a, b| b.cmp(a));
        if let Some(v) = versions.first() {
            return dir.join(v).to_string_lossy().into_owned();
        }
    }
    // Gradle wrapper cache
    let wrapper_dir = PathBuf::from(&home).join(".gradle/wrapper/dists");
    if let Ok(entries) = std::fs::read_dir(&wrapper_dir) {
        let mut versions: Vec<String> = entries
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                if e.path().is_dir() && name.starts_with("gradle-") && name.ends_with("-bin") {
                    Some(name)
                } else {
                    None
                }
            })
            .collect();
        versions.sort_by(|a, b| b.cmp(a));
        if let Some(dist_name) = versions.first() {
            // The actual gradle home is one level deeper
            let dist_path = wrapper_dir.join(dist_name);
            if let Ok(hash_entries) = std::fs::read_dir(&dist_path) {
                for hash_entry in hash_entries.flatten() {
                    let gradle_home = hash_entry.path().join(dist_name.trim_end_matches("-bin"));
                    if gradle_home.is_dir() {
                        return gradle_home.to_string_lossy().into_owned();
                    }
                }
            }
        }
    }
    String::new()
}

fn find_java_home() -> String {
    if let Ok(v) = std::env::var("JAVA_HOME") {
        if !v.is_empty() && Path::new(&v).is_dir() {
            return v;
        }
    }
    // Derive from `java` binary via readlink -f
    if let Ok(java_path) = which::which("java") {
        let resolved = std::fs::canonicalize(&java_path).unwrap_or(java_path);
        // resolved is like /usr/lib/jvm/java-25-openjdk/bin/java
        // JAVA_HOME is two levels up
        if let Some(bin_dir) = resolved.parent() {
            if let Some(java_home) = bin_dir.parent() {
                if java_home.join("bin/javac").exists() || java_home.join("bin/java").exists() {
                    return java_home.to_string_lossy().into_owned();
                }
            }
        }
    }
    // Scan /usr/lib/jvm/ — prefer Java 21 (LTS, compatible with cordova-android 13's Gradle 8.7)
    // Java 25 class files (major version 69) are not supported by Gradle 8.7.
    let jvm_dir = Path::new("/usr/lib/jvm");
    if jvm_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(jvm_dir) {
            let mut jdks: Vec<String> = entries
                .flatten()
                .filter_map(|e| {
                    let path = e.path();
                    if path.is_dir() && path.join("bin/javac").exists() {
                        Some(path.to_string_lossy().into_owned())
                    } else {
                        None
                    }
                })
                .collect();
            // Sort descending — but penalise java-25+ to prefer java-21
            jdks.sort_by(|a, b| {
                let score = |s: &str| -> i32 {
                    if s.contains("java-21") { 100 }
                    else if s.contains("java-17") { 90 }
                    else if s.contains("java-11") { 80 }
                    else { 50 }
                };
                score(b).cmp(&score(a))
            });
            if let Some(jdk) = jdks.first() {
                return jdk.clone();
            }
        }
    }
    String::new()
}

fn find_binary(name: &str) -> String {
    which::which(name)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn find_cordova() -> String {
    let home = home_dir();
    let candidates = [
        format!("{home}/.npm-global/bin/cordova"),
        format!("{home}/.local/bin/cordova"),
        "/usr/local/bin/cordova".to_string(),
        "/usr/bin/cordova".to_string(),
    ];
    candidates
        .iter()
        .find(|p| Path::new(p.as_str()).is_file())
        .cloned()
        .or_else(|| which::which("cordova").ok().map(|p| p.to_string_lossy().into_owned()))
        .unwrap_or_default()
}

fn find_build_tools_version(sdk_path: &str) -> String {
    if sdk_path.is_empty() {
        return "34.0.0".into();
    }
    let bt_dir = Path::new(sdk_path).join("build-tools");
    if let Ok(entries) = std::fs::read_dir(&bt_dir) {
        let mut versions: Vec<String> = entries
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                if e.path().is_dir() { Some(name) } else { None }
            })
            .collect();
        // Semver-ish sort descending: prefer 34.x over 35+ for compatibility
        versions.sort_by(|a, b| b.cmp(a));
        // Prefer 34.x for apksigner compatibility
        if let Some(v34) = versions.iter().find(|v| v.starts_with("34.")) {
            return v34.clone();
        }
        if let Some(v) = versions.first() {
            return v.clone();
        }
    }
    "34.0.0".into()
}

fn default_output_dir() -> String {
    let home = home_dir();
    let downloads = format!("{home}/Downloads");
    if Path::new(&downloads).is_dir() {
        downloads
    } else {
        home
    }
}

fn find_renpy_sdk() -> String {
    let home = home_dir();
    // Glob ~/renpy-*/ for the newest version
    if let Ok(entries) = std::fs::read_dir(&home) {
        let mut sdks: Vec<String> = entries
            .flatten()
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().into_owned();
                let path = e.path();
                if name.starts_with("renpy-") && path.is_dir() && path.join("renpy.sh").exists() {
                    Some(path.to_string_lossy().into_owned())
                } else {
                    None
                }
            })
            .collect();
        sdks.sort_by(|a, b| b.cmp(a)); // newest version first
        if let Some(sdk) = sdks.first() {
            return sdk.clone();
        }
    }
    // Common install paths
    let candidates = [
        format!("{home}/.local/share/renpy"),
        "/opt/renpy".into(),
    ];
    candidates
        .iter()
        .find(|p| Path::new(p.as_str()).join("renpy.sh").exists())
        .cloned()
        .or_else(|| {
            which::which("renpy")
                .ok()
                .and_then(|p| p.parent().map(|d| d.to_string_lossy().into_owned()))
        })
        .unwrap_or_default()
}

fn home_dir() -> String {
    std::env::var("HOME").unwrap_or_else(|_| "/tmp".into())
}
