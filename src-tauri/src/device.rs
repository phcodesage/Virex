//! A stable, anonymous per-install identifier.
//!
//! The API proxy counts free-tier usage per device. This is deliberately a
//! random value generated on first run — not a hardware serial — so it says
//! nothing about the user or their machine.

use std::{fs, io::Read, path::PathBuf};

use crate::config;

/// Path of the file holding the device id.
fn device_path() -> PathBuf {
    config::config_dir().join("device-id")
}

/// The device id for this install, creating and persisting one on first call.
///
/// Falls back to an ephemeral id if the file can't be written, which just means
/// the free-tier counter resets — never a hard failure.
pub fn id() -> String {
    let path = device_path();

    if let Ok(existing) = fs::read_to_string(&path) {
        let trimmed = existing.trim();
        if trimmed.len() >= 16 {
            return trimmed.to_string();
        }
    }

    let fresh = random_hex();
    if let Some(dir) = path.parent() {
        let _ = fs::create_dir_all(dir);
    }
    if let Err(e) = fs::write(&path, &fresh) {
        log::warn!("couldn't persist device id: {e}");
    }
    fresh
}

/// 32 hex chars of OS randomness. Uses `/dev/urandom` so we don't pull in a
/// crypto dependency for what is only an opaque counter key.
fn random_hex() -> String {
    let mut buf = [0u8; 16];
    match fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut buf)) {
        Ok(()) => {}
        Err(_) => {
            // Last-resort fallback; still unique enough for a usage counter.
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0);
            buf.copy_from_slice(&nanos.to_le_bytes()[..16.min(16)]);
        }
    }
    buf.iter().map(|b| format!("{b:02x}")).collect()
}
