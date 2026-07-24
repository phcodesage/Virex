//! User settings, persisted as TOML in the app config directory.

use serde::{Deserialize, Serialize};
use std::fs;

use crate::config;

/// The default system prompt used to steer the rewrite.
pub const DEFAULT_SYSTEM_PROMPT: &str = "You are an expert writing assistant.\n\nRules:\n- Preserve the original meaning.\n- Never explain your changes.\n- Return ONLY the rewritten text.\n- Improve grammar and clarity.\n- Preserve links and markdown.\n- Keep emojis and formatting.";

/// The user-facing theme preference.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    System,
    Light,
    Dark,
}

/// Persisted application settings. Field names use camelCase on the wire so the
/// TypeScript frontend can consume them directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    /// DeepSeek model id. The API key is stored separately in the Keychain.
    pub model: String,
    pub temperature: f32,
    pub system_prompt: String,
    /// Tauri accelerator string, e.g. `CmdOrCtrl+Shift+P`.
    pub shortcut: String,
    pub launch_at_login: bool,
    pub auto_replace: bool,
    pub auto_copy: bool,
    pub show_notifications: bool,
    pub theme: Theme,
    pub timeout_secs: u64,
    pub max_retries: u32,
    /// OpenAI-compatible base URL for the DeepSeek API.
    pub base_url: String,
    #[serde(default = "default_target_lang")]
    pub target_lang: String,
}

fn default_target_lang() -> String {
    "en".into()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            // `deepseek-chat` is DeepSeek's documented general chat model and a
            // safe default. Change to `deepseek-v4-flash` in Settings if your
            // account exposes it.
            model: "deepseek-chat".into(),
            temperature: 0.7,
            system_prompt: DEFAULT_SYSTEM_PROMPT.into(),
            // `CmdOrCtrl` = the Command key on macOS, which is what the
            // physical Windows key maps to. More reliable than `Super`.
            shortcut: "CmdOrCtrl+Shift+1".into(),
            launch_at_login: false,
            auto_replace: true,
            auto_copy: false,
            show_notifications: true,
            theme: Theme::System,
            timeout_secs: 30,
            max_retries: 1,
            base_url: "https://api.deepseek.com".into(),
            target_lang: "en".into(),
        }
    }
}

impl Settings {
    /// Load settings from disk, falling back to defaults when the file is
    /// missing or malformed.
    pub fn load() -> Self {
        let path = config::settings_path();
        match fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text).unwrap_or_else(|e| {
                log::warn!("settings.toml malformed ({e}); using defaults");
                Settings::default()
            }),
            Err(_) => Settings::default(),
        }
    }

    /// Persist settings to disk, creating the config directory if needed.
    pub fn save(&self) -> anyhow::Result<()> {
        let dir = config::config_dir();
        fs::create_dir_all(&dir)?;
        let text = toml::to_string_pretty(self)?;
        fs::write(config::settings_path(), text)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_roundtrip_through_toml() {
        let original = Settings::default();
        let text = toml::to_string_pretty(&original).unwrap();
        let parsed: Settings = toml::from_str(&text).unwrap();
        assert_eq!(parsed.model, original.model);
        assert_eq!(parsed.shortcut, original.shortcut);
        assert_eq!(parsed.theme, original.theme);
        assert!((parsed.temperature - original.temperature).abs() < f32::EPSILON);
    }

    #[test]
    fn serializes_camel_case_for_frontend() {
        let text = serde_json::to_string(&Settings::default()).unwrap();
        assert!(text.contains("\"systemPrompt\""));
        assert!(text.contains("\"launchAtLogin\""));
        assert!(text.contains("\"baseUrl\""));
    }
}
