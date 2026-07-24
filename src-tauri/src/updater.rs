//! "Check for Updates…" — downloads and installs in place.
//!
//! Backed by `tauri-plugin-updater`, which verifies each update against the
//! public key baked into `tauri.conf.json` before installing, so a tampered or
//! third-party build can't be pushed to users.

use std::time::Duration;

use tauri::AppHandle;
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_updater::UpdaterExt;

/// How long after launch the automatic check runs. Long enough that it never
/// competes with building the tray and warming the overlay.
const LAUNCH_CHECK_DELAY: Duration = Duration::from_secs(10);

/// Check for a newer release and, with the user's consent, install it.
///
/// Driven by the tray's "Check for Updates…", so it reports every outcome —
/// silence in response to a click reads as a broken menu item.
pub fn check(app: &AppHandle) {
    spawn_check(app, true);
}

/// The same check, run shortly after launch without being asked.
///
/// This one stays quiet unless there is genuinely something to install: an
/// "up to date" or "the network is down" dialog on every launch would be an
/// interruption the user never asked for.
pub fn check_on_launch(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(LAUNCH_CHECK_DELAY).await;
        spawn_check(&app, false);
    });
}

fn spawn_check(app: &AppHandle, announce: bool) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let current = app.package_info().version.to_string();

        let updater = match app.updater() {
            Ok(u) => u,
            Err(e) => {
                log::warn!("updater unavailable: {e}");
                if announce {
                    report_error(&app, format!("Couldn't start the updater.\n\n{e}"));
                }
                return;
            }
        };

        match updater.check().await {
            Ok(Some(update)) => prompt_and_install(app, update, current).await,
            Ok(None) => {
                log::info!("no update available (running {current})");
                if announce {
                    app.dialog()
                        .message(format!("You're on the latest version ({current})."))
                        .title("Virex is up to date")
                        .kind(MessageDialogKind::Info)
                        .show(|_| {});
                }
            }
            Err(e) => {
                log::warn!("update check failed: {e}");
                if announce {
                    report_error(&app, format!("Couldn't check for updates.\n\n{e}"));
                }
            }
        }
    });
}

async fn prompt_and_install(app: AppHandle, update: tauri_plugin_updater::Update, current: String) {
    let version = update.version.clone();

    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog()
        .message(format!(
            "Virex {version} is available — you're on {current}.\n\nDownload and install it now?"
        ))
        .title("Update available")
        .buttons(MessageDialogButtons::OkCancelCustom(
            "Install".into(),
            "Later".into(),
        ))
        .show(move |install| {
            let _ = tx.send(install);
        });

    if !matches!(rx.await, Ok(true)) {
        return;
    }

    log::info!("downloading update {version}");
    match update.download_and_install(|_, _| {}, || {}).await {
        Ok(()) => {
            let app_for_restart = app.clone();
            app.dialog()
                .message(format!(
                    "Virex {version} is installed. Restart now to use it?"
                ))
                .title("Update installed")
                .buttons(MessageDialogButtons::OkCancelCustom(
                    "Restart".into(),
                    "Later".into(),
                ))
                .show(move |restart| {
                    if restart {
                        app_for_restart.restart();
                    }
                });
        }
        Err(e) => {
            log::warn!("update install failed: {e}");
            report_error(&app, format!("The update couldn't be installed.\n\n{e}"));
        }
    }
}

fn report_error(app: &AppHandle, message: String) {
    app.dialog()
        .message(message)
        .title("Update failed")
        .kind(MessageDialogKind::Error)
        .show(|_| {});
}
