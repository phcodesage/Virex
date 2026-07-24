//! Shared, thread-safe application state.

use std::sync::Mutex;

use crate::settings::Settings;

/// Global mutable state guarded behind a mutex and stored in Tauri's state map.
pub struct AppState {
    inner: Mutex<Inner>,
}

struct Inner {
    settings: Settings,
    /// The text captured from the frontmost app on the last hotkey press.
    last_selection: Option<String>,
    /// PID of the app that was frontmost when Virex was last triggered, so we
    /// can re-activate it before pasting.
    frontmost_pid: Option<i32>,
    /// When true, the hotkey is ignored (tray "Pause").
    paused: bool,
}

impl AppState {
    pub fn new(settings: Settings) -> Self {
        Self {
            inner: Mutex::new(Inner {
                settings,
                last_selection: None,
                frontmost_pid: None,
                paused: false,
            }),
        }
    }

    pub fn settings(&self) -> Settings {
        self.inner.lock().expect("state poisoned").settings.clone()
    }

    pub fn set_settings(&self, settings: Settings) {
        self.inner.lock().expect("state poisoned").settings = settings;
    }

    pub fn last_selection(&self) -> Option<String> {
        self.inner
            .lock()
            .expect("state poisoned")
            .last_selection
            .clone()
    }

    pub fn set_last_selection(&self, text: Option<String>) {
        self.inner.lock().expect("state poisoned").last_selection = text;
    }

    pub fn frontmost_pid(&self) -> Option<i32> {
        self.inner.lock().expect("state poisoned").frontmost_pid
    }

    pub fn set_frontmost_pid(&self, pid: Option<i32>) {
        self.inner.lock().expect("state poisoned").frontmost_pid = pid;
    }

    pub fn is_paused(&self) -> bool {
        self.inner.lock().expect("state poisoned").paused
    }

    pub fn set_paused(&self, paused: bool) {
        self.inner.lock().expect("state poisoned").paused = paused;
    }
}
