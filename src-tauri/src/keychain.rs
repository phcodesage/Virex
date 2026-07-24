//! Secure storage of the DeepSeek API key in the macOS Keychain.
//!
//! The key is never written to disk in plaintext and never logged.

use anyhow::Result;
use keyring::Entry;

use crate::config::{KEYCHAIN_ACCOUNT, KEYCHAIN_SERVICE};

fn entry() -> Result<Entry> {
    Ok(Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)?)
}

/// Store (or overwrite) the API key in the Keychain.
pub fn set_api_key(key: &str) -> Result<()> {
    entry()?.set_password(key)?;
    Ok(())
}

/// Fetch the API key, returning `None` if none has been stored yet.
pub fn get_api_key() -> Option<String> {
    match entry() {
        Ok(e) => match e.get_password() {
            Ok(k) => Some(k),
            Err(keyring::Error::NoEntry) => None,
            Err(e) => {
                log::warn!("keychain read failed: {e}");
                None
            }
        },
        Err(_) => None,
    }
}

/// Whether an API key is currently stored.
pub fn has_api_key() -> bool {
    get_api_key().is_some()
}

fn license_entry() -> Result<Entry> {
    Ok(Entry::new(
        KEYCHAIN_SERVICE,
        crate::config::KEYCHAIN_LICENSE_ACCOUNT,
    )?)
}

/// Store the user's Pro licence key.
pub fn set_license(key: &str) -> Result<()> {
    license_entry()?.set_password(key)?;
    Ok(())
}

/// Fetch the Pro licence key, if one has been entered.
pub fn get_license() -> Option<String> {
    match license_entry() {
        Ok(e) => match e.get_password() {
            Ok(k) if !k.trim().is_empty() => Some(k),
            _ => None,
        },
        Err(_) => None,
    }
}

/// Remove the stored licence (used when downgrading / signing out).
pub fn clear_license() -> Result<()> {
    match license_entry()?.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

/// If no key is stored yet but `DEEPSEEK_API_KEY` is present in the environment
/// (e.g. loaded from a dev `.env`), seed the Keychain with it. This lets
/// developers drop a key into `.env` without a plaintext key living long-term.
pub fn seed_from_env_if_empty() {
    if has_api_key() {
        return;
    }
    if let Ok(key) = std::env::var("DEEPSEEK_API_KEY") {
        let key = key.trim();
        if !key.is_empty() {
            if let Err(e) = set_api_key(key) {
                log::warn!("failed to seed key from env: {e}");
            } else {
                log::info!("seeded API key from environment into Keychain");
            }
        }
    }
}
