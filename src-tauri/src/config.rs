//! Application-wide constants and filesystem paths.

use std::path::PathBuf;

/// Bundle identifier, used for config dir and Keychain service name.
pub const APP_ID: &str = "com.virex.app";

/// GitHub `owner/repo` whose Releases are checked by "Check for Updates…".
/// Releases must be tagged `vX.Y.Z` with the DMG attached.
pub const GITHUB_REPO: &str = "phcodesage/Virex";

/// Keychain service + accounts. The DeepSeek key no longer lives on the client
/// (the API proxy holds it); we only store the user's Pro licence.
pub const KEYCHAIN_SERVICE: &str = "com.virex.app";
pub const KEYCHAIN_ACCOUNT: &str = "deepseek-api-key";
pub const KEYCHAIN_LICENSE_ACCOUNT: &str = "virex-license";

/// Virex API proxy (Cloudflare Worker) that holds the DeepSeek key and enforces
/// plan limits. Overridable via `api_base` in settings.toml so the endpoint can
/// be repointed without a rebuild.
pub const DEFAULT_API_BASE: &str = "https://virex-api.rechceltoledo.workers.dev";

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
