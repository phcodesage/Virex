//! Logging configuration.
//!
//! IMPORTANT: prompts, selected text, rewritten output, and API keys must never
//! be logged. Only lifecycle/diagnostic messages go through `log`.

use tauri::{plugin::TauriPlugin, Wry};
use tauri_plugin_log::{Target, TargetKind};

/// Build the logging plugin (stdout + rotating file under the app log dir).
pub fn plugin() -> TauriPlugin<Wry> {
    tauri_plugin_log::Builder::new()
        .level(log::LevelFilter::Info)
        .targets([
            Target::new(TargetKind::Stdout),
            Target::new(TargetKind::LogDir { file_name: None }),
        ])
        .build()
}
