//! Application-wide constants and filesystem paths.

use std::path::PathBuf;

/// Bundle identifier, used for config dir and Keychain service name.
pub const APP_ID: &str = "com.virex.app";

/// GitHub `owner/repo` whose Releases are checked by "Check for Updates…".
/// Releases must be tagged `vX.Y.Z` with the DMG attached.
pub const GITHUB_REPO: &str = "phcodesage/Virex";

/// Keychain service + account under which the DeepSeek API key is stored.
pub const KEYCHAIN_SERVICE: &str = "com.virex.app";
pub const KEYCHAIN_ACCOUNT: &str = "deepseek-api-key";

/// Label of the floating overlay window.
pub const OVERLAY_LABEL: &str = "overlay";
/// Label of the settings window.
pub const SETTINGS_LABEL: &str = "settings";

/// Event name the backend emits streaming rewrite updates on.
pub const STREAM_EVENT: &str = "virex://stream";

/// Directory where `settings.toml` lives (`~/Library/Application Support/com.virex.app`).
pub fn config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(APP_ID)
}

/// Full path to the settings file.
pub fn settings_path() -> PathBuf {
    config_dir().join("settings.toml")
}
