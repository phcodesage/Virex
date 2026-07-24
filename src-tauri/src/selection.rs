//! Capturing the user's currently-selected text from the frontmost app.

use anyhow::{anyhow, Result};

use crate::input;

/// Capture the selected text in the frontmost application.
///
/// Strategy: preserve the existing clipboard, synthesize ⌘C, read the resulting
/// clipboard, then restore the original clipboard so we don't clobber it. Must
/// run while the *source* app is still frontmost (i.e. before the overlay is
/// shown/focused).
pub fn capture_selection() -> Result<String> {
    let previous = input::clipboard_text();

    // The trigger shortcut (e.g. Cmd+Shift+1) is likely still held. Wait for the
    // user to lift those keys, otherwise the synthesized ⌘C arrives as
    // Cmd+Shift+C (which, e.g., opens the Colors panel) instead of copying.
    input::wait_modifiers_clear();

    // Copy the selection, then poll the clipboard until it changes. Catalyst
    // apps (e.g. WhatsApp) publish to the pasteboard noticeably slower than
    // native AppKit apps, so a single fixed wait misses them.
    input::send_copy()?;
    let captured = input::wait_for_clipboard_change(previous.as_deref(), 1200);

    log::info!(
        "capture: prev={:?} bytes, after-copy={:?} bytes",
        previous.as_ref().map(|s| s.len()),
        captured.as_ref().map(|s| s.len()),
    );

    // Restore the user's original clipboard contents.
    if let Some(prev) = previous.clone() {
        let _ = input::set_clipboard_text(&prev);
    }

    match captured {
        // The clipboard didn't change after ⌘C. Either nothing was selected, or
        // the synthesized keystroke was dropped (usually a missing/stale
        // Accessibility permission on a dev build). Fail loudly rather than
        // silently operating on the previous clipboard contents.
        Some(text) if Some(&text) == previous.as_ref() => {
            Err(anyhow!("clipboard unchanged after copy"))
        }
        Some(text) if !text.trim().is_empty() => Ok(text),
        _ => Err(anyhow!("no text selected")),
    }
}
