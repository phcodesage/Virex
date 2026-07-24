import { getVersion } from "@tauri-apps/api/app";
import { type Component, createSignal, onCleanup, onMount, Show } from "solid-js";
import * as api from "../lib/api";

/**
 * Setup window: the two things Virex needs to work — Accessibility permission
 * (to read and replace the selection) and a DeepSeek API key (bring your own).
 * Model, prompt, and other tuning live in the app defaults / settings.toml.
 */
export const Settings: Component = () => {
  const [trusted, setTrusted] = createSignal(true);
  const [version, setVersion] = createSignal("");
  const [apiKey, setApiKey] = createSignal("");
  const [keyStored, setKeyStored] = createSignal(false);
  const [savingKey, setSavingKey] = createSignal(false);
  const [keyError, setKeyError] = createSignal("");
  const [keySaved, setKeySaved] = createSignal(false);
  let poll: number | undefined;

  const refresh = async () => {
    const ok = await api.accessibilityTrusted();
    setTrusted(ok);
    return ok;
  };

  // Fire the native macOS prompt (the system dialog), then keep polling so the
  // UI flips to "granted" the instant the toggle is switched on — no manual
  // refresh needed.
  const requestAccess = async () => {
    await api.requestAccessibility();
    if (poll === undefined) {
      poll = window.setInterval(async () => {
        if (await refresh()) {
          window.clearInterval(poll);
          poll = undefined;
        }
      }, 1000);
    }
  };

  const saveKey = async () => {
    const key = apiKey().trim();
    if (!key) return;
    setSavingKey(true);
    setKeyError("");
    try {
      await api.setApiKey(key);
      setApiKey("");
      setKeyStored(true);
      setKeySaved(true);
      setTimeout(() => setKeySaved(false), 2000);
    } catch (e) {
      setKeyError(String(e));
    } finally {
      setSavingKey(false);
    }
  };

  onMount(async () => {
    getVersion().then(setVersion).catch(() => {});
    setKeyStored(await api.hasApiKey());
    const ok = await refresh();
    // Ask straight away on open if we're not trusted yet.
    if (!ok) void requestAccess();
  });

  onCleanup(() => {
    if (poll !== undefined) window.clearInterval(poll);
  });

  return (
    <div class="mx-auto max-w-md space-y-5 p-8 text-black/90 dark:text-white/90">
      <header class="flex items-center gap-3">
        <div class="flex h-9 w-9 items-center justify-center rounded-xl bg-blue-500 text-lg font-bold text-white">
          V
        </div>
        <div>
          <h1 class="text-lg font-semibold">
            Virex{" "}
            <Show when={version()}>
              <span class="text-xs font-medium opacity-40">v{version()}</span>
            </Show>
          </h1>
          <p class="text-xs opacity-50">AI writing assistant, anywhere</p>
        </div>
      </header>

      {/* 1 — Accessibility permission */}
      <Show
        when={!trusted()}
        fallback={
          <div class="rounded-xl border border-green-500/30 bg-green-500/10 p-4 text-sm">
            <p class="font-medium text-green-600 dark:text-green-400">
              Accessibility granted ✓
            </p>
            <p class="mt-1 opacity-70">
              Virex can read and replace your selected text.
            </p>
          </div>
        }
      >
        <div class="rounded-xl border border-amber-500/30 bg-amber-500/10 p-4 text-sm">
          <p class="font-medium">Accessibility permission required</p>
          <p class="mt-1 opacity-70">
            Virex needs Accessibility access to read and replace selected text.
          </p>
          <ol class="mt-2 list-decimal space-y-0.5 pl-4 opacity-70">
            <li>Click <span class="font-medium">Grant Access</span> below.</li>
            <li>In the dialog, choose <span class="font-medium">Open System Settings</span>.</li>
            <li>Toggle <span class="font-medium">Virex</span> on.</li>
          </ol>
          <p class="mt-2 text-xs opacity-50">
            macOS requires that final toggle for security — this window updates
            automatically once it's on.
          </p>
          <div class="mt-3 flex items-center gap-2">
            <button
              onClick={requestAccess}
              class="rounded-lg bg-amber-500 px-3 py-1.5 text-xs font-medium text-white hover:bg-amber-600"
            >
              Grant Access
            </button>
            <button
              onClick={() => api.openAccessibilitySettings()}
              class="rounded-lg px-3 py-1.5 text-xs font-medium text-black/60 hover:bg-black/5 dark:text-white/60 dark:hover:bg-white/10"
            >
              Open System Settings
            </button>
          </div>
        </div>
      </Show>

      {/* 2 — DeepSeek API key */}
      <div class="space-y-2">
        <div class="flex items-baseline justify-between">
          <label class="text-sm font-medium">DeepSeek API Key</label>
          <Show when={keyStored()}>
            <span class="text-xs text-green-600 dark:text-green-400">Saved ✓</span>
          </Show>
        </div>
        <input
          type="password"
          value={apiKey()}
          onInput={(e) => setApiKey(e.currentTarget.value)}
          onKeyDown={(e) => e.key === "Enter" && saveKey()}
          placeholder={keyStored() ? "•••••••••• (stored in Keychain)" : "sk-…"}
          class="vx-input"
          autocomplete="off"
          spellcheck={false}
        />
        <div class="flex items-center gap-2">
          <button
            onClick={saveKey}
            disabled={!apiKey().trim() || savingKey()}
            class="rounded-lg bg-blue-500 px-3 py-1.5 text-xs font-medium text-white hover:bg-blue-600 disabled:cursor-not-allowed disabled:opacity-40"
          >
            {savingKey() ? "Saving…" : keyStored() ? "Replace key" : "Save key"}
          </button>
          <Show when={keySaved()}>
            <span class="text-xs text-green-500">Key saved to Keychain</span>
          </Show>
          <Show when={keyError()}>
            <span class="text-xs text-red-500">{keyError()}</span>
          </Show>
        </div>
        <p class="text-xs opacity-50">
          Virex uses your own key, so your text goes straight to DeepSeek — never
          through our servers. Create one at platform.deepseek.com, then paste it
          here. It's stored in the macOS Keychain.
          {keyStored() ? " Leave blank to keep the current key." : ""}
        </p>
      </div>

      <Show when={trusted() && keyStored()}>
        <div class="rounded-xl border border-blue-500/20 bg-blue-500/5 p-3 text-xs opacity-80">
          You're all set — select text anywhere and press your shortcut.
        </div>
      </Show>
    </div>
  );
};
