// Thin typed wrappers around Tauri commands + event channels.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Settings, StreamEvent, TranslationResult } from "./types";

export function getSettings(): Promise<Settings> {
  return invoke("get_settings");
}

export function saveSettings(settings: Settings): Promise<void> {
  return invoke("save_settings", { settings });
}

export function hasApiKey(): Promise<boolean> {
  return invoke("has_api_key");
}

export function setApiKey(key: string): Promise<void> {
  return invoke("set_api_key", { key });
}

export function accessibilityTrusted(): Promise<boolean> {
  return invoke("accessibility_trusted");
}

export function requestAccessibility(): Promise<void> {
  return invoke("request_accessibility");
}

export function openAccessibilitySettings(): Promise<void> {
  return invoke("open_accessibility_settings");
}

/** Re-run the improvement on the last captured selection. */
export function retry(): Promise<void> {
  return invoke("retry_last");
}

/** Replace the originally-selected text in the source app with `text`. */
export function replaceSelection(text: string): Promise<void> {
  return invoke("replace_selection", { text });
}

export function copyToClipboard(text: string): Promise<void> {
  return invoke("copy_to_clipboard", { text });
}

/** Hide the floating overlay. */
export function closeOverlay(): Promise<void> {
  return invoke("close_overlay");
}

/** Translate given text to target language using Google translate web API, then paraphrase using DeepSeek. */
export function translateMessage(
  text: string,
  targetLang?: string
): Promise<TranslationResult> {
  return invoke("translate_message", { text, targetLang });
}

/** Translate the last captured selection and stream to overlay. */
export function translateSelection(targetLang?: string): Promise<void> {
  return invoke("translate_selection", { targetLang });
}

/** Subscribe to streaming rewrite events. Returns an unlisten fn. */
export function onStream(cb: (e: StreamEvent) => void): Promise<UnlistenFn> {
  return listen<StreamEvent>("virex://stream", (event) => cb(event.payload));
}
