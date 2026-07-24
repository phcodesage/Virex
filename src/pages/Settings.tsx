import { getVersion } from "@tauri-apps/api/app";
import { type Component, createSignal, onCleanup, onMount, Show } from "solid-js";
import * as api from "../lib/api";

/**
 * Permission-only settings window. Its sole job is to get Virex trusted for
 * Accessibility (needed to read and replace the selection). Model, prompt, and
 * other config live in the app defaults / settings.toml, not here.
 */
export const Settings: Component = () => {
  const [trusted, setTrusted] = createSignal(true);
  const [version, setVersion] = createSignal("");
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

  onMount(async () => {
    getVersion().then(setVersion).catch(() => {});
    const ok = await refresh();
    // Ask straight away on open if we're not trusted yet.
    if (!ok) void requestAccess();
  });

  onCleanup(() => {
    if (poll !== undefined) window.clearInterval(poll);
  });

  return (
    <div class="mx-auto max-w-md space-y-6 p-8 text-black/90 dark:text-white/90">
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

      <Show
        when={!trusted()}
        fallback={
          <div class="rounded-xl border border-green-500/30 bg-green-500/10 p-4 text-sm">
            <p class="font-medium text-green-600 dark:text-green-400">
              Accessibility granted ✓
            </p>
            <p class="mt-1 opacity-70">
              Virex is ready. Select text anywhere and press your shortcut.
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
    </div>
  );
};
