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

/// Position the overlay near the mouse cursor and show it.
pub fn show(app: &AppHandle) -> Result<()> {
    let window = app
        .get_webview_window(OVERLAY_LABEL)
        .ok_or_else(|| anyhow::anyhow!("overlay window missing"))?;

    if let Some((x, y)) = cursor_point() {
        let (px, py) = placement(x, y);
        let _ = window.set_position(LogicalPosition::new(px, py));
    }

    window.show()?;
    window.set_focus()?;
    Ok(())
}

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
