//! macOS status-bar (menu bar) tray icon and menu.

use anyhow::Result;
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
    AppHandle, Manager,
};

use crate::{hotkeys, state::AppState, updater, window};

/// Build the status-bar item with its menu.
pub fn build(app: &AppHandle) -> Result<()> {
    let open = MenuItem::with_id(app, "open_settings", "Open Settings…", true, None::<&str>)?;
    let pause = MenuItem::with_id(app, "pause", "Pause", true, None::<&str>)?;
    let resume = MenuItem::with_id(app, "resume", "Resume", true, None::<&str>)?;
    let updates =
        MenuItem::with_id(app, "check_updates", "Check for Updates…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Virex", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;

    let menu = Menu::with_items(
        app,
        &[&open, &sep1, &pause, &resume, &sep2, &updates, &quit],
    )?;

    // A dedicated template icon: transparent apart from the V glyph. macOS
    // template rendering keys off the alpha channel, so the full-bleed app icon
    // (95% opaque) came out as a solid block in the menu bar.
    let icon = Image::from_bytes(include_bytes!("../icons/tray.png"))?;

    TrayIconBuilder::with_id("virex-tray")
        .icon(icon)
        .icon_as_template(true) // render monochrome like a native menu-bar icon
        .tooltip("Virex")
        .menu(&menu)
        .on_menu_event(move |app, event| handle_menu(app, event.id.as_ref()))
        .build(app)?;

    Ok(())
}

fn handle_menu(app: &AppHandle, id: &str) {
    match id {
        "open_settings" => window::open_settings(app),
        "pause" => {
            app.state::<AppState>().set_paused(true);
            hotkeys::unregister_all(app);
        }
        "resume" => {
            let state = app.state::<AppState>();
            state.set_paused(false);
            let accel = state.settings().shortcut;
            if let Err(e) = hotkeys::register(app, &accel) {
                log::error!("failed to re-register shortcut: {e}");
            }
        }
        "check_updates" => updater::check(app),
        "quit" => app.exit(0),
        _ => {}
    }
}
