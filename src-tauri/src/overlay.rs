//! Creation, positioning, and visibility of the floating overlay window.

use anyhow::Result;
use tauri::{AppHandle, LogicalPosition, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::config::OVERLAY_LABEL;

const WIDTH: f64 = 420.0;
const HEIGHT: f64 = 260.0;

/// Build the overlay window up-front (hidden). Called once during setup so the
/// webview is warm and the first ⌘⇧P feels instant.
pub fn build(app: &AppHandle) -> Result<()> {
    if app.get_webview_window(OVERLAY_LABEL).is_some() {
        return Ok(());
    }
    WebviewWindowBuilder::new(app, OVERLAY_LABEL, WebviewUrl::App("index.html".into()))
        .title("Virex")
        .inner_size(WIDTH, HEIGHT)
        .decorations(false)
        .transparent(true)
        .shadow(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .build()?;
    Ok(())
}

/// Position the overlay near the mouse cursor and show it — *without* stealing
/// key focus.
///
/// Focus is deliberately left with the source app: the pipeline synthesizes ⌘C
/// to capture the selection right after this, and that keystroke must land in
/// the still-frontmost source app. Call [`focus`] once capture is done to make
/// the overlay the key window. See [`crate::selection::capture_selection`].
pub fn show(app: &AppHandle) -> Result<()> {
    let window = app
        .get_webview_window(OVERLAY_LABEL)
        .ok_or_else(|| anyhow::anyhow!("overlay window missing"))?;

    // Only (re)position when the overlay is actually appearing. If it's already
    // on screen — e.g. Translate/Retry fired from a button inside it — leave it
    // where it is, so it doesn't jump to the cursor or snap back after a drag.
    let already_visible = window.is_visible().unwrap_or(false);
    if !already_visible {
        // Prefer anchoring to the selected text's on-screen rect; fall back to
        // the mouse cursor when Accessibility can't report it (e.g. browsers).
        if let Some((rx, ry, rw, rh)) = crate::axselect::selected_text_bounds() {
            let (px, py) = placement_for_rect(rx, ry, rw, rh);
            let _ = window.set_position(LogicalPosition::new(px, py));
        } else if let Some((x, y)) = cursor_point() {
            let (px, py) = placement(x, y);
            let _ = window.set_position(LogicalPosition::new(px, py));
        }
    }

    window.show()?;
    Ok(())
}

/// Make the overlay the key window so keyboard input (Esc/Enter, typing) is
/// handled immediately.
///
/// On an accessory app, `set_focus` alone doesn't reliably make the window key
/// right away, so the first keystrokes lag. Activating our own app first makes
/// it key immediately. Call this *after* the selection has been captured, never
/// before — activating steals focus from the source app.
pub fn focus(app: &AppHandle) -> Result<()> {
    if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
        // AppKit activation and window focus must happen on the main thread —
        // the global-shortcut handler calls us from a background thread, where
        // activation silently no-ops and the app never steals key focus from
        // e.g. WhatsApp.
        let win = window.clone();
        let _ = window.run_on_main_thread(move || {
            // 1. Make Virex the active app.
            activate_self();
            // 2. Make the overlay the key window.
            let _ = win.set_focus();
            // 3. Give the WKWebView DOM focus. Making the window key doesn't
            //    guarantee the webview is first responder, so the JS `keydown`
            //    listener (Enter → Replace, Esc → close) may otherwise never
            //    fire and the keystroke lands in the app underneath instead.
            let _ = win.eval("window.focus();");
        });
    }
    Ok(())
}

/// Bring Virex itself to the foreground so the overlay becomes the key window
/// without the accessory-app focus delay. Must be called on the main thread.
#[cfg(target_os = "macos")]
fn activate_self() {
    use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication};
    let app = NSRunningApplication::currentApplication();
    // `ActivateAllWindows` brings the overlay forward with us. (The older
    // `ignoringOtherApps` flag is deprecated and a no-op on macOS 14+.)
    app.activateWithOptions(NSApplicationActivationOptions::ActivateAllWindows);
}

#[cfg(not(target_os = "macos"))]
fn activate_self() {}

/// Hide the overlay. On an accessory app this returns key focus to the
/// previously-frontmost application.
pub fn hide(app: &AppHandle) -> Result<()> {
    if let Some(window) = app.get_webview_window(OVERLAY_LABEL) {
        window.hide()?;
    }
    Ok(())
}

/// Compute the overlay's top-left position (logical points) so the window sits
/// *above* the cursor, horizontally centred on it, clamped to the screen.
/// Falls back to below the cursor when there isn't room above.
fn placement(x: f64, y: f64) -> (f64, f64) {
    const GAP: f64 = 14.0;
    const MARGIN: f64 = 8.0;
    let (sx, sy, sw, sh) = screen_bounds();

    // Horizontal: centre on the cursor, clamped within the screen.
    let px = (x - WIDTH / 2.0).clamp(sx + MARGIN, sx + sw - WIDTH - MARGIN);

    // Vertical: prefer above the cursor; if it would clip the top, go below.
    let above = y - HEIGHT - GAP;
    let py = if above >= sy + MARGIN {
        above
    } else {
        (y + GAP + 8.0).min(sy + sh - HEIGHT - MARGIN)
    };

    (px, py)
}

/// Compute the overlay's top-left position so it sits just *above* the selected
/// text's rectangle, horizontally centred on it, clamped to the screen. Falls
/// back to just below the selection when there isn't room above.
fn placement_for_rect(rx: f64, ry: f64, rw: f64, rh: f64) -> (f64, f64) {
    const GAP: f64 = 10.0;
    const MARGIN: f64 = 8.0;
    let (sx, sy, sw, sh) = screen_bounds();

    let center_x = rx + rw / 2.0;
    let px = (center_x - WIDTH / 2.0).clamp(sx + MARGIN, sx + sw - WIDTH - MARGIN);

    let above = ry - HEIGHT - GAP;
    let py = if above >= sy + MARGIN {
        above
    } else {
        (ry + rh + GAP).min(sy + sh - HEIGHT - MARGIN)
    };

    (px, py)
}

/// Main display bounds in logical points (origin, size). Defaults to a common
/// resolution if Quartz is unavailable.
#[cfg(target_os = "macos")]
fn screen_bounds() -> (f64, f64, f64, f64) {
    use core_graphics::display::CGDisplay;
    let b = CGDisplay::main().bounds();
    (b.origin.x, b.origin.y, b.size.width, b.size.height)
}

#[cfg(not(target_os = "macos"))]
fn screen_bounds() -> (f64, f64, f64, f64) {
    (0.0, 0.0, 1440.0, 900.0)
}

/// Current mouse location in logical screen points (top-left origin), via
/// Quartz. Returns `None` off macOS or if the event source can't be created.
#[cfg(target_os = "macos")]
fn cursor_point() -> Option<(f64, f64)> {
    use core_graphics::event::CGEvent;
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let source = CGEventSource::new(CGEventSourceStateID::CombinedSessionState).ok()?;
    let event = CGEvent::new(source).ok()?;
    let p = event.location();
    Some((p.x, p.y))
}

#[cfg(not(target_os = "macos"))]
fn cursor_point() -> Option<(f64, f64)> {
    None
}
