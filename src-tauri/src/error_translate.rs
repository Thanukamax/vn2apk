/// Translate a raw build error message into a human-friendly string.
/// Falls back to the original message if no pattern matches.
pub fn translate(raw: &str) -> String {
    if raw.contains("keystore password was incorrect") {
        return "Wrong keystore password. Re-enter your store/key password.".into();
    }
    if raw.contains("Failed to read key") {
        return "Could not read the signing key — check your keystore alias and password.".into();
    }
    if raw.contains("RAPT not found")
        || (raw.contains("android_build") && raw.contains("not a directory"))
    {
        return "RAPT is not installed. Open the Ren'Py Launcher → Android → Install SDK.".into();
    }
    if raw.contains("SDK location not found") || raw.contains("ANDROID_HOME") {
        return "Android SDK path is not configured. Set it in Settings → Android SDK.".into();
    }
    if raw.contains("JAVA_HOME")
        || raw.contains("java: not found")
        || (raw.contains("No such file") && raw.contains("java"))
    {
        return "Java (JDK) not found. Set JAVA_HOME in Settings or install JDK 17+.".into();
    }
    if raw.contains("No space left") {
        return "Not enough disk space. Free up space in /tmp and try again.".into();
    }
    if raw.contains("Permission denied") {
        return "Permission denied. Check that the output folder is writable.".into();
    }
    if raw.contains("Could not find") && raw.contains("apk") {
        return "APK not found after build. Check the build log for errors.".into();
    }
    if raw.contains("exited with exit status: 255") {
        return "Build tool crashed unexpectedly. Check the full log above for the root cause."
            .into();
    }
    raw.to_string()
}
