//! "Check for Updates…": compares the running version against the latest
//! GitHub Release and, if newer, offers to open the download page.

use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

use crate::config::GITHUB_REPO;

/// Check GitHub Releases for a newer version and report the result in a dialog.
/// Runs the network request off the UI thread.
pub fn check(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let current = app.package_info().version.clone();
        match latest_release().await {
            Ok((latest, page_url)) if latest > current => {
                app.dialog()
                    .message(format!(
                        "Virex {latest} is available — you're on {current}."
                    ))
                    .title("Update available")
                    .buttons(MessageDialogButtons::OkCancelCustom(
                        "Download".into(),
                        "Later".into(),
                    ))
                    .show(move |download| {
                        if download {
                            open_url(&page_url);
                        }
                    });
            }
            Ok((_latest, _)) => {
                app.dialog()
                    .message(format!("You're on the latest version ({current})."))
                    .title("Virex is up to date")
                    .kind(MessageDialogKind::Info)
                    .show(|_| {});
            }
            Err(e) => {
                log::warn!("update check failed: {e}");
                app.dialog()
                    .message(format!("Couldn't check for updates.\n\n{e}"))
                    .title("Update check failed")
                    .kind(MessageDialogKind::Error)
                    .show(|_| {});
            }
        }
    });
}

/// Fetch the latest release's version (from its `vX.Y.Z` tag) and web page URL.
async fn latest_release() -> anyhow::Result<(semver::Version, String)> {
    if GITHUB_REPO.starts_with("OWNER/") {
        anyhow::bail!("No update repository is configured yet.");
    }

    let url = format!("https://api.github.com/repos/{GITHUB_REPO}/releases/latest");
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Virex-Updater")
        .build()?;

    let resp = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("GitHub returned {}", resp.status());
    }

    let json: serde_json::Value = resp.json().await?;
    let tag = json
        .get("tag_name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("release had no tag_name"))?;
    let version = semver::Version::parse(tag.trim_start_matches('v'))?;
    let page = json
        .get("html_url")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    Ok((version, page))
}

/// Open a URL in the user's default browser.
#[cfg(target_os = "macos")]
fn open_url(url: &str) {
    let _ = std::process::Command::new("open").arg(url).spawn();
}

#[cfg(not(target_os = "macos"))]
fn open_url(_url: &str) {}
