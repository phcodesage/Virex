//! Global shortcut registration.

use std::str::FromStr;

use anyhow::{anyhow, Result};
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::pipeline;

/// Register `accel` (Tauri accelerator syntax) as the trigger shortcut,
/// clearing any previously-registered shortcuts first.
pub fn register(app: &AppHandle, accel: &str) -> Result<()> {
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();

    let shortcut =
        Shortcut::from_str(accel).map_err(|_| anyhow!("invalid shortcut: {accel}"))?;

    gs.on_shortcut(shortcut, move |app, _shortcut, event| {
        if event.state == ShortcutState::Pressed {
            pipeline::trigger(app.clone());
        }
    })
    .map_err(|e| anyhow!("failed to register shortcut: {e}"))?;

    log::info!("registered global shortcut: {accel}");
    Ok(())
}

/// Remove all registered shortcuts (used when pausing).
pub fn unregister_all(app: &AppHandle) {
    let _ = app.global_shortcut().unregister_all();
}
