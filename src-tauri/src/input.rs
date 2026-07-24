//! Low-level input helpers: clipboard access and simulated ⌘C / ⌘V.
//!
//! Synthesizes native system copy (⌘C) and paste (⌘V) shortcuts, which
//! every text field on macOS honours.

use std::{thread, time::Duration};

use anyhow::{anyhow, Result};
use arboard::Clipboard;
use enigo::{Direction, Enigo, Key, Keyboard, Settings as EnigoSettings};

/// Read the current clipboard text, if any.
pub fn clipboard_text() -> Option<String> {
    Clipboard::new().ok()?.get_text().ok()
}

/// Poll the clipboard until its text differs from `baseline`, or `timeout_ms`
/// elapses. Returns the final clipboard contents.
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

/// Send native macOS key combination (Command + KeyCode).
/// Keycode 0x08 = 'C', Keycode 0x09 = 'V'.
#[cfg(target_os = "macos")]
fn send_cmd_key(virtual_key: u16) -> Result<()> {
    use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| anyhow!("failed to create CGEventSource"))?;

    let event_down = CGEvent::new_keyboard_event(source.clone(), virtual_key, true)
        .map_err(|_| anyhow!("failed to create down event"))?;
    event_down.set_flags(CGEventFlags::CGEventFlagCommand);
    event_down.post(CGEventTapLocation::HID);

    thread::sleep(Duration::from_millis(35));

    let event_up = CGEvent::new_keyboard_event(source, virtual_key, false)
        .map_err(|_| anyhow!("failed to create up event"))?;
    event_up.set_flags(CGEventFlags::CGEventFlagCommand);
    event_up.post(CGEventTapLocation::HID);

    thread::sleep(Duration::from_millis(35));
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn send_cmd_key(virtual_key: u16) -> Result<()> {
    let letter = if virtual_key == 0x08 { 'c' } else { 'v' };
    cmd_chord(letter)
}

#[allow(dead_code)]
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

/// Block (up to ~800 ms) until the user lifts trigger/modifier keys.
pub fn wait_modifiers_clear() {
    for _ in 0..40 {
        if !modifiers_held() {
            thread::sleep(Duration::from_millis(20));
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
}

/// Simulate ⌘C to copy the current selection into the clipboard.
pub fn send_copy() -> Result<()> {
    wait_modifiers_clear();
    send_cmd_key(0x08)
}

/// Simulate ⌘V to paste clipboard contents at the current insertion point.
pub fn send_paste() -> Result<()> {
    wait_modifiers_clear();
    send_cmd_key(0x09)
}

/// Settle delay so target app registers a synthesized shortcut.
pub fn settle() {
    thread::sleep(Duration::from_millis(150));
}
