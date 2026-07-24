//! Update checking.
//!
//! Stub for the MVP: wire this to `tauri-plugin-updater` with a signed release
//! endpoint when distribution is set up. For now it just informs the user.

use tauri::AppHandle;

use crate::notify;

/// Check for updates (placeholder).
pub fn check(app: &AppHandle) {
    notify::send(
        app,
        "Virex is up to date",
        "You're running the latest version.",
    );
}
