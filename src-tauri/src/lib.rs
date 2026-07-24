//! Virex — a native macOS AI writing assistant.
//!
//! Press the global shortcut anywhere, and Virex captures the current text
//! selection, streams an improved version from DeepSeek into a floating
//! overlay, and pastes it back over the selection on Enter.

mod accessibility;
mod axselect;
mod commands;
mod config;
mod deepseek;
mod events;
mod frontmost;
mod hotkeys;
mod input;
mod keychain;
mod logging;
mod notify;
mod overlay;
mod pipeline;
mod replace;
mod selection;
mod settings;
mod state;
mod tray;
mod translator;
mod updater;
mod window;

use tauri_plugin_autostart::MacosLauncher;

use crate::{settings::Settings, state::AppState};

/// Build and run the Tauri application.
pub fn run() {
    // Load a dev `.env` (e.g. DEEPSEEK_API_KEY) if present. Never required in
    // production, where the key lives in the Keychain.
    let _ = dotenvy::dotenv();

    let settings = Settings::load();

    tauri::Builder::default()
        .plugin(logging::plugin())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(AppState::new(settings.clone()))
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::has_api_key,
            commands::set_api_key,
            commands::accessibility_trusted,
            commands::request_accessibility,
            commands::open_accessibility_settings,
            commands::retry_last,
            commands::replace_selection,
            commands::copy_to_clipboard,
            commands::close_overlay,
            commands::translate_message,
            commands::translate_selection,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();

            // Menu-bar-only utility: no dock icon, and hiding the overlay
            // returns focus to the previously-active app.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Seed the Keychain from a dev `.env` key on first run.
            keychain::seed_from_env_if_empty();

            // Warm the overlay webview so the first trigger is instant.
            if let Err(e) = overlay::build(&handle) {
                log::error!("overlay build failed: {e}");
            }

            // Register the global shortcut.
            if let Err(e) = hotkeys::register(&handle, &settings.shortcut) {
                log::error!("shortcut registration failed: {e}");
            }

            // Build the status-bar tray.
            if let Err(e) = tray::build(&handle) {
                log::error!("tray build failed: {e}");
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Virex")
        .run(|_app, event| {
            // Keep running in the background even when all windows are closed;
            // the app lives in the menu bar.
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                api.prevent_exit();
            }
        });
}
