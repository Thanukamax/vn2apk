# vn2apk

> ⚠️ **Work in progress** — early development, expect rough edges and missing features.

A desktop app that converts PC visual novel / RPG game folders into signed Android APKs. Drop a folder, configure your keystore, click Build.

---

## Supported engines

| Engine | Status |
|---|---|
| RPG Maker MV | ✅ Working |
| RPG Maker MZ | ✅ Working |
| TyranoBuilder | ✅ Working |
| Ren'Py | ✅ Working (requires RAPT setup, see below) |
| NScripter / KiriKiri | ⏳ Not yet supported |

---

## Requirements

You need these tools installed and configured in **Settings → SDK & Tools**:

- **JDK 21** — `java -version` should report 21.x
- **Android SDK** — with `build-tools;34.0.0`, `platform-tools`, `platforms;android-34`
- **Gradle 8.7**
- **Node.js 20+** (for RPG Maker / TyranoBuilder builds via Cordova)
- **Cordova 13** — `npm install -g cordova`
- **Keystore** — generated in Settings → Keystore, or bring your own

For **Ren'Py games** specifically:
- **Ren'Py SDK 8.5.x** — download from [renpy.org](https://www.renpy.org/latest.html)
- **RAPT** (Ren'Py Android Packaging Tool) — installed via the Ren'Py Launcher:
  1. Open the Ren'Py Launcher: `~/renpy-8.5.2-sdk/renpy.sh`
  2. Click **Android → Install SDK**
  3. Follow the prompts

The **Toolchain** tab in the app can auto-install most tools for you. Click **Install** next to any missing tool.

---

## Quick start

```bash
# Install deps
bun install

# Run in dev mode
bun run tauri dev

# Build a release binary
bun run tauri build
```

---

## How it works

1. **Drop** a game folder onto the Build tab
2. The app **detects** the engine automatically
3. Fill in the **App ID**, name, and version
4. Enter your **keystore passwords** and click Build
5. The signed APK lands in your configured output directory (default: `~/Downloads`)

---

## Project layout

```
src/             React + TypeScript frontend
src-tauri/       Rust backend (Tauri v2)
  src/
    pipeline/    Build pipelines per engine
    toolchain/   Tool detection & install
    settings.rs  Persistent settings via tauri-plugin-store
```

---

## Known limitations

- Linux only tested so far; Windows support is in progress
- Ren'Py builds can take 5–15 minutes on first run (Gradle downloads dependencies)
- No AAB / Play Store bundle support yet
- No over-the-air update or auto-sign workflow yet

---

## License

MIT
