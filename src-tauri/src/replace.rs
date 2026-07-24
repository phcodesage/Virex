//! Replacing the selected text in the source app with the rewritten text.

use std::{thread, time::Duration};
use anyhow::Result;

use crate::input;

/// Replace the current selection in the frontmost app with `text`.
///
/// Assumes the source app still holds the selection (the overlay is
/// non-activating, so the source app remains frontmost). Places `text` on the
/// clipboard and synthesizes ⌘V, which overwrites the highlighted selection.
/// The prior clipboard contents are restored afterwards.
pub fn replace_selection(text: &str) -> Result<()> {
    let previous = input::clipboard_text();

    input::set_clipboard_text(text)?;
    input::settle();
    input::send_paste()?;
    
    // Allow target app time to process paste before restoring previous content
    thread::sleep(Duration::from_millis(350));

    if let Some(prev) = previous {
        let _ = input::set_clipboard_text(&prev);
    }
    Ok(())
}
