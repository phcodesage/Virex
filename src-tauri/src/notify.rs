//! Thin wrapper over the system notification plugin, gated by the user's
//! "Show notifications" preference.

use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::state::AppState;

/// Show a system notification, unless the user has disabled them.
pub fn send(app: &AppHandle, title: &str, body: &str) {
    if !app.state::<AppState>().settings().show_notifications {
        return;
    }
    let _ = app
        .notification()
        .builder()
        .title(title)
        .body(body)
        .show();
}
