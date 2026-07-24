// Thin typed wrappers around Tauri commands + event channels.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Settings, StreamEvent } from "./types";

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

/** What the API proxy reports about this device's plan and remaining quota. */
export interface PlanInfo {
  plan: "free" | "pro";
  used: number;
  limit: number | null;
  remaining: number | null;
  /** Licence is valid but already active on the maximum number of devices. */
  seatLimited: boolean;
  seatsUsed: number;
  maxSeats: number;
}

/** Current plan + today's usage. */
export function getPlan(): Promise<PlanInfo> {
  return invoke("get_plan");
}

/** Store (or clear, when blank) a Pro licence key; returns the resulting plan. */
export function setLicense(key: string): Promise<PlanInfo> {
  return invoke("set_license", { key });
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

/** Open a URL in the default browser. */
export function openUrl(url: string): Promise<void> {
  return invoke("open_url", { url });
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

/** Subscribe to streaming rewrite events. Returns an unlisten fn. */
export function onStream(cb: (e: StreamEvent) => void): Promise<UnlistenFn> {
  return listen<StreamEvent>("virex://stream", (event) => cb(event.payload));
}
