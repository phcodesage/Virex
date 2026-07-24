//! macOS Accessibility permission detection and prompting.
//!
//! Virex needs Accessibility access to synthesize ⌘C / ⌘V into other apps.

/// Whether this process is currently trusted for Accessibility.
#[cfg(target_os = "macos")]
pub fn is_trusted() -> bool {
    macos_accessibility_client::accessibility::application_is_trusted()
}

/// Prompt the user for Accessibility access (shows the system dialog once).
#[cfg(target_os = "macos")]
pub fn prompt_trust() -> bool {
    macos_accessibility_client::accessibility::application_is_trusted_with_prompt()
}

/// Open the Accessibility pane in System Settings.
#[cfg(target_os = "macos")]
pub fn open_settings() {
    let _ = std::process::Command::new("open")
        .arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility")
        .spawn();
}

#[cfg(not(target_os = "macos"))]
pub fn is_trusted() -> bool {
    true
}
#[cfg(not(target_os = "macos"))]
pub fn prompt_trust() -> bool {
    true
}
#[cfg(not(target_os = "macos"))]
pub fn open_settings() {}
