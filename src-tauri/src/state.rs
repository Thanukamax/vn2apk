use crate::settings::AppSettings;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

pub struct AppState {
    pub settings: Mutex<AppSettings>,
    pub build_cancel: Arc<AtomicBool>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            settings: Mutex::new(AppSettings::default()),
            build_cancel: Arc::new(AtomicBool::new(false)),
        }
    }
}
