// Shared types mirrored from the Rust `settings` module.

export type Theme = "system" | "light" | "dark";

export interface Settings {
  model: string;
  temperature: number;
  systemPrompt: string;
  shortcut: string;
  launchAtLogin: boolean;
  autoReplace: boolean;
  autoCopy: boolean;
  showNotifications: boolean;
  theme: Theme;
  timeoutSecs: number;
  maxRetries: number;
  baseUrl: string;
  targetLang: string;
}

export interface TranslationResult {
  success: boolean;
  translation: string;
  rawTranslation: string;
  detectedLanguage: string;
  targetLanguage: string;
}

// Streaming events emitted by the backend to the overlay window.
export type StreamEvent =
  | { kind: "start"; original: string }
  | { kind: "delta"; text: string }
  | { kind: "done"; full: string }
  | { kind: "error"; message: string };
