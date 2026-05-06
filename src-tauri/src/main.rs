#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // Workaround: Intel iris (Mesa) crashes in i915_batch_submit when WebKit creates
    // EGL sync objects during compositing on some Fedora/Nobara versions.
    // Disabling the DMA-buf renderer avoids that code path entirely.
    #[cfg(target_os = "linux")]
    {
        if std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").is_err() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
    }
    vn2apk_lib::run();
}
