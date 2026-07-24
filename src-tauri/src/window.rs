//! Settings window management.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::config::SETTINGS_LABEL;

/// Show the settings window, creating it lazily on first use.
pub fn open_settings(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(SETTINGS_LABEL) {
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }

    let built = WebviewWindowBuilder::new(
        app,
        SETTINGS_LABEL,
        WebviewUrl::App("index.html".into()),
    )
    .title("Virex Settings")
    .inner_size(560.0, 720.0)
    .min_inner_size(480.0, 560.0)
    .resizable(true)
    .center()
    .build();

    match built {
        Ok(window) => {
            let _ = window.set_focus();
        }
        Err(e) => log::error!("failed to open settings window: {e}"),
    }
}
