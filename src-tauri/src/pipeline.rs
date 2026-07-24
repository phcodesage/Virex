//! The end-to-end "improve my selection" flow, triggered by the hotkey or a
//! Retry request.

use tauri::{AppHandle, Manager};

use crate::{
    accessibility, deepseek, events::StreamEvent, frontmost, keychain, notify, overlay, selection,
    state::AppState,
};

/// Run the full pipeline: capture selection, show overlay, stream a rewrite.
///
/// Runs the (blocking) capture on the calling thread, then spawns the async
/// network work. Safe to call from the global-shortcut handler.
pub fn trigger(app: AppHandle) {
    let state = app.state::<AppState>();

    if state.is_paused() {
        return;
    }

    // Remember which app is frontmost *now* (before the overlay steals focus)
    // so we can re-activate it before pasting the replacement.
    let pid = frontmost::current_pid();
    state.set_frontmost_pid(pid);

    log::info!(
        "hotkey triggered (accessibility trusted = {}, source pid = {:?})",
        accessibility::is_trusted(),
        pid
    );

    // Gate on Accessibility permission.
    if !accessibility::is_trusted() {
        accessibility::prompt_trust();
        crate::window::open_settings(&app);
        notify::send(
            &app,
            "Accessibility required",
            "Grant Virex access in System Settings, then try again.",
        );
        return;
    }

    // Show overlay IMMEDIATELY so appearance and focus are instant (<1ms).
    if let Err(e) = overlay::show(&app) {
        log::error!("failed to show overlay: {e}");
        return;
    }

    // Capture the current selection while the source app is still frontmost.
    let selected = match selection::capture_selection() {
        Ok(text) => {
            log::info!("captured selection ({} chars)", text.chars().count());
            text
        }
        Err(e) => {
            log::warn!("capture failed: {e}");
            notify::send(
                &app,
                "Couldn't read the selection",
                "Highlight text first. If it keeps failing, re-grant Virex in System Settings › Privacy › Accessibility.",
            );
            return;
        }
    };

    state.set_last_selection(Some(selected.clone()));

    rewrite(app, selected);
}

/// Re-run the rewrite on the last captured selection (Retry).
pub fn retry(app: AppHandle) {
    let selection = app.state::<AppState>().last_selection();
    if let Some(text) = selection {
        let _ = overlay::show(&app);
        rewrite(app, text);
    }
}

/// Spawn the async streaming rewrite for `input`, emitting stream events.
fn rewrite(app: AppHandle, input: String) {
    StreamEvent::Start {
        original: input.clone(),
    }
    .emit(&app);

    let api_key = match keychain::get_api_key() {
        Some(k) => k,
        None => {
            StreamEvent::Error {
                message: "No API key. Add your DeepSeek key in Settings.".into(),
            }
            .emit(&app);
            crate::window::open_settings(&app);
            return;
        }
    };

    let settings = app.state::<AppState>().settings();

    tauri::async_runtime::spawn(async move {
        let app_for_delta = app.clone();
        let result = deepseek::stream_rewrite(
            deepseek::RewriteRequest {
                api_key: &api_key,
                settings: &settings,
                input: &input,
            },
            |delta| {
                StreamEvent::Delta {
                    text: delta.to_string(),
                }
                .emit(&app_for_delta);
            },
        )
        .await;

        match result {
            Ok(full) => {
                if settings.auto_copy {
                    let _ = crate::input::set_clipboard_text(&full);
                }
                StreamEvent::Done { full }.emit(&app);
            }
            Err(e) => {
                StreamEvent::Error {
                    message: e.to_string(),
                }
                .emit(&app);
            }
        }
    });
}

/// Run translation on text or captured selection and stream result to overlay.
pub fn translate(app: AppHandle, input: String, target_lang: Option<String>) {
    let target_lang = target_lang
        .unwrap_or_else(|| app.state::<AppState>().settings().target_lang.clone());

    StreamEvent::Start {
        original: input.clone(),
    }
    .emit(&app);

    let api_key = keychain::get_api_key().unwrap_or_default();
    let settings = app.state::<AppState>().settings();

    tauri::async_runtime::spawn(async move {
        let app_for_delta = app.clone();
        let result = crate::translator::translate_and_paraphrase(
            &api_key,
            &settings,
            &input,
            &target_lang,
            |delta| {
                StreamEvent::Delta {
                    text: delta.to_string(),
                }
                .emit(&app_for_delta);
            },
        )
        .await;

        match result {
            Ok(res) => {
                if settings.auto_copy {
                    let _ = crate::input::set_clipboard_text(&res.translation);
                }
                StreamEvent::Done {
                    full: res.translation,
                }
                .emit(&app);
            }
            Err(e) => {
                StreamEvent::Error {
                    message: e.to_string(),
                }
                .emit(&app);
            }
        }
    });
}
