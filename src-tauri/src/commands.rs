//! Tauri command handlers exposed to the frontend.

use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;

use crate::{
    accessibility, hotkeys, input, keychain, overlay, pipeline, settings::Settings,
    state::AppState,
};

type CmdResult<T = ()> = Result<T, String>;

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> Settings {
    state.settings()
}

#[tauri::command]
pub fn save_settings(app: AppHandle, state: State<AppState>, settings: Settings) -> CmdResult {
    let previous = state.settings();

    settings.save().map_err(|e| e.to_string())?;
    state.set_settings(settings.clone());

    // Re-register the shortcut if it changed (and we're not paused).
    if settings.shortcut != previous.shortcut && !state.is_paused() {
        hotkeys::register(&app, &settings.shortcut).map_err(|e| e.to_string())?;
    }

    // Apply launch-at-login.
    if settings.launch_at_login != previous.launch_at_login {
        let autolaunch = app.autolaunch();
        let res = if settings.launch_at_login {
            autolaunch.enable()
        } else {
            autolaunch.disable()
        };
        if let Err(e) = res {
            log::warn!("autostart toggle failed: {e}");
        }
    }

    Ok(())
}

#[tauri::command]
pub fn has_api_key() -> bool {
    keychain::has_api_key()
}

#[tauri::command]
pub fn set_api_key(key: String) -> CmdResult {
    keychain::set_api_key(&key).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn accessibility_trusted() -> bool {
    accessibility::is_trusted()
}

#[tauri::command]
pub fn request_accessibility() -> bool {
    accessibility::prompt_trust()
}

#[tauri::command]
pub fn open_accessibility_settings() {
    accessibility::open_settings();
}

#[tauri::command]
pub fn retry_last(app: AppHandle) {
    pipeline::retry(app);
}

#[tauri::command]
pub fn copy_to_clipboard(text: String) -> CmdResult {
    input::set_clipboard_text(&text).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn close_overlay(app: AppHandle) -> CmdResult {
    overlay::hide(&app).map_err(|e| e.to_string())
}

/// Hide the overlay, wait for focus to return to the source app, then paste the
/// rewritten text over the original selection.
///
/// The paste runs on the main thread: AppKit clipboard/event injection is
/// unreliable (and can crash) from a background thread.
#[tauri::command]
pub async fn replace_selection(
    app: AppHandle,
    state: State<'_, AppState>,
    text: String,
) -> CmdResult {
    log::info!("replace: hiding overlay, {} chars", text.chars().count());
    let pid = state.frontmost_pid();
    overlay::hide(&app).map_err(|e| e.to_string())?;

    app.run_on_main_thread(move || {
        // Re-activate the source app so the paste has a key window to land in,
        // then paste. Runs on the main thread — AppKit + event injection are
        // unreliable off the main thread.
        if let Some(pid) = pid {
            crate::frontmost::activate(pid);
        }
        input::settle();
        input::settle();
        if let Err(e) = crate::replace::replace_selection(&text) {
            log::warn!("replace failed: {e}");
        } else {
            log::info!("replace: pasted");
        }
    })
    .map_err(|e| e.to_string())
}
