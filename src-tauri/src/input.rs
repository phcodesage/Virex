//! Low-level input helpers: clipboard access and simulated ⌘C / ⌘V.
//!
//! These drive the "robust everywhere" text capture/replace strategy: rather
//! than relying on per-app Accessibility text APIs (which are unreliable in
//! Chrome, Electron, and many editors), we synthesize the system copy/paste
//! shortcuts, which every text field on macOS honours.

use std::{thread, time::Duration};

use anyhow::{anyhow, Result};
use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings as EnigoSettings};

/// Read the current clipboard text, if any.
pub fn clipboard_text() -> Option<String> {
    Clipboard::new().ok()?.get_text().ok()
}

/// Poll the clipboard until its text differs from `baseline`, or `timeout_ms`
/// elapses. Returns the final clipboard contents. Used after a synthesized ⌘C to
/// accommodate apps (notably Catalyst) that write the pasteboard slowly.
pub fn wait_for_clipboard_change(baseline: Option<&str>, timeout_ms: u64) -> Option<String> {
    let step = 50u64;
    let mut latest = clipboard_text();
    let mut elapsed = 0u64;
    while elapsed < timeout_ms {
        if latest.as_deref() != baseline {
            return latest;
        }
        thread::sleep(Duration::from_millis(step));
        elapsed += step;
        latest = clipboard_text();
    }
    latest
}

/// Overwrite the clipboard with `text`.
pub fn set_clipboard_text(text: &str) -> Result<()> {
    Clipboard::new()
        .map_err(|e| anyhow!("clipboard unavailable: {e}"))?
        .set_text(text.to_owned())
        .map_err(|e| anyhow!("clipboard write failed: {e}"))
}

/// Press ⌘ + `letter` as a single chord, with small gaps so slow apps register
/// the modifier before the key.
fn cmd_chord(letter: char) -> Result<()> {
    let gap = Duration::from_millis(30);
    let mut enigo =
        Enigo::new(&EnigoSettings::default()).map_err(|e| anyhow!("input backend: {e}"))?;
    enigo
        .key(Key::Meta, Direction::Press)
        .map_err(|e| anyhow!("key press: {e}"))?;
    thread::sleep(gap);
    enigo
        .key(Key::Unicode(letter), Direction::Click)
        .map_err(|e| anyhow!("key click: {e}"))?;
    thread::sleep(gap);
    enigo
        .key(Key::Meta, Direction::Release)
        .map_err(|e| anyhow!("key release: {e}"))?;
    Ok(())
}

/// Whether any keyboard modifier (Shift/Control/Alt/Command) is physically held
/// right now, per the live Quartz event state.
#[cfg(target_os = "macos")]
fn modifiers_held() -> bool {
    use core_graphics::event::{CGEvent, CGEventFlags};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let Ok(source) = CGEventSource::new(CGEventSourceStateID::CombinedSessionState) else {
        return false;
    };
    let Ok(event) = CGEvent::new(source) else {
        return false;
    };
    let flags = event.get_flags();
    flags.intersects(
        CGEventFlags::CGEventFlagShift
            | CGEventFlags::CGEventFlagControl
            | CGEventFlags::CGEventFlagAlternate
            | CGEventFlags::CGEventFlagCommand,
    )
}

#[cfg(not(target_os = "macos"))]
fn modifiers_held() -> bool {
    false
}

/// Block (up to ~800 ms) until the user lifts the trigger-shortcut modifier
/// keys, so a following synthesized ⌘C isn't polluted by a stray Cmd/Shift.
pub fn wait_modifiers_clear() {
    for _ in 0..40 {
        if !modifiers_held() {
            // A hair of extra time after the last key comes up.
            thread::sleep(Duration::from_millis(20));
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
}

/// Simulate ⌘C to copy the current selection into the clipboard.
pub fn send_copy() -> Result<()> {
    cmd_chord('c')
}

/// Simulate ⌘V to paste clipboard contents at the current insertion point.
pub fn send_paste() -> Result<()> {
    cmd_chord('v')
}

/// Small settle delay so the target app registers a synthesized shortcut before
/// we read/write the clipboard again.
pub fn settle() {
    thread::sleep(Duration::from_millis(120));
}
