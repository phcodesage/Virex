//! Tracking and re-activating the application that was frontmost when Virex was
//! triggered, so a synthesized paste lands back in it.
//!
//! Showing the overlay makes Virex the active app; without explicitly
//! re-activating the source app, a following ⌘V has no key window to land in.

/// The process id of the currently-frontmost application, if any.
#[cfg(target_os = "macos")]
pub fn current_pid() -> Option<i32> {
    use objc2_app_kit::NSWorkspace;
    let workspace = NSWorkspace::sharedWorkspace();
    let app = workspace.frontmostApplication()?;
    Some(app.processIdentifier())
}

/// Bring the app with `pid` back to the foreground.
#[cfg(target_os = "macos")]
pub fn activate(pid: i32) {
    use objc2_app_kit::{NSApplicationActivationOptions, NSRunningApplication};
    if let Some(app) = NSRunningApplication::runningApplicationWithProcessIdentifier(pid) {
        let activated =
            app.activateWithOptions(NSApplicationActivationOptions::ActivateAllWindows);
        log::info!("reactivated source app pid={pid} ok={activated}");
    } else {
        log::warn!("source app pid={pid} no longer running");
    }
}

#[cfg(not(target_os = "macos"))]
pub fn current_pid() -> Option<i32> {
    None
}

#[cfg(not(target_os = "macos"))]
pub fn activate(_pid: i32) {}
