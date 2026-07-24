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
mod device;
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
        .plugin(tauri_plugin_updater::Builder::new().build())
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
            commands::set_license,
            commands::get_plan,
            commands::accessibility_trusted,
            commands::request_accessibility,
            commands::open_accessibility_settings,
            commands::open_url,
            commands::retry_last,
            commands::replace_selection,
            commands::copy_to_clipboard,
            commands::close_overlay,
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

            // Look for a new version without being asked. Quiet unless there's
            // something to install.
            updater::check_on_launch(&handle);

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Virex")
        .run(|_app, event| {
            if let tauri::RunEvent::ExitRequested { code, api, .. } = event {
                // `code` is None when the last window merely closed — the app
                // lives in the menu bar, so keep running. It's Some(_) when
                // something deliberately asked us to quit (the tray's Quit
                // item), and blocking that made Quit do nothing at all.
                if code.is_none() {
                    api.prevent_exit();
                }
            }
        });
}
