import {
  type Component,
  createSignal,
  onMount,
  Show,
} from "solid-js";
import * as api from "../lib/api";
import type { Settings as SettingsData, Theme } from "../lib/types";

const MODELS = ["deepseek-chat", "deepseek-reasoner", "deepseek-v4-flash"];

/** Full settings window. */
export const Settings: Component = () => {
  const [s, setS] = createSignal<SettingsData>();
  const [apiKey, setApiKeyInput] = createSignal("");
  const [keyStored, setKeyStored] = createSignal(false);
  const [trusted, setTrusted] = createSignal(true);
  const [saved, setSaved] = createSignal(false);

  onMount(async () => {
    setS(await api.getSettings());
    setKeyStored(await api.hasApiKey());
    setTrusted(await api.accessibilityTrusted());
  });

  const patch = (p: Partial<SettingsData>) => setS((cur) => ({ ...cur!, ...p }));

  const save = async () => {
    await api.saveSettings(s()!);
    if (apiKey().trim()) {
      await api.setApiKey(apiKey().trim());
      setApiKeyInput("");
      setKeyStored(true);
    }
    setSaved(true);
    setTimeout(() => setSaved(false), 1500);
  };

  return (
    <Show when={s()} fallback={<div class="p-8 text-sm opacity-50">Loading…</div>}>
      <div class="mx-auto max-w-xl space-y-6 p-8 text-black/90 dark:text-white/90">
        <header class="flex items-center gap-3">
          <div class="flex h-9 w-9 items-center justify-center rounded-xl bg-blue-500 text-lg font-bold text-white">
            V
          </div>
          <div>
            <h1 class="text-lg font-semibold">Virex Settings</h1>
            <p class="text-xs opacity-50">AI writing assistant, anywhere</p>
          </div>
        </header>

        <Show when={!trusted()}>
          <div class="rounded-xl border border-amber-500/30 bg-amber-500/10 p-4 text-sm">
            <p class="font-medium">Accessibility permission required</p>
            <p class="mt-1 opacity-70">
              Virex needs Accessibility access to read and replace selected text.
            </p>
            <button
              onClick={() => api.openAccessibilitySettings()}
              class="mt-2 rounded-lg bg-amber-500 px-3 py-1.5 text-xs font-medium text-white"
            >
              Open System Settings
            </button>
          </div>
        </Show>

        <Field label="DeepSeek API Key">
          <input
            type="password"
            placeholder={keyStored() ? "•••••••••• (stored in Keychain)" : "sk-…"}
            value={apiKey()}
            onInput={(e) => setApiKeyInput(e.currentTarget.value)}
            class="vx-input"
          />
          <p class="mt-1 text-xs opacity-50">
            Stored securely in the macOS Keychain. Leave blank to keep the current key.
          </p>
        </Field>

        <Field label="Model">
          <select
            value={s()!.model}
            onChange={(e) => patch({ model: e.currentTarget.value })}
            class="vx-input"
          >
            {MODELS.map((m) => (
              <option value={m}>{m}</option>
            ))}
          </select>
        </Field>

        <Field label={`Temperature — ${s()!.temperature.toFixed(2)}`}>
          <input
            type="range"
            min="0"
            max="1.5"
            step="0.05"
            value={s()!.temperature}
            onInput={(e) => patch({ temperature: +e.currentTarget.value })}
            class="w-full accent-blue-500"
          />
        </Field>

        <Field label="System Prompt">
          <textarea
            rows="6"
            value={s()!.systemPrompt}
            onInput={(e) => patch({ systemPrompt: e.currentTarget.value })}
            class="vx-input font-mono text-xs leading-relaxed"
          />
        </Field>

        <Field label="Global Shortcut">
          <input
            value={s()!.shortcut}
            onInput={(e) => patch({ shortcut: e.currentTarget.value })}
            placeholder="Super+Shift+1"
            class="vx-input"
          />
          <p class="mt-1 text-xs opacity-50">
            Uses Tauri accelerator syntax, e.g. <code>CmdOrCtrl+Shift+P</code>.
          </p>
        </Field>

        <Field label="Translation Target Language">
          <input
            value={s()!.targetLang || "en"}
            onInput={(e) => patch({ targetLang: e.currentTarget.value })}
            placeholder="en"
            class="vx-input"
          />
          <p class="mt-1 text-xs opacity-50">
            Language code for Google Translate (e.g. <code>en</code>, <code>es</code>, <code>fr</code>, <code>de</code>, <code>zh-CN</code>, <code>ja</code>).
          </p>
        </Field>

        <Field label="Theme">
          <div class="flex gap-2">
            {(["system", "light", "dark"] as Theme[]).map((t) => (
              <button
                onClick={() => patch({ theme: t })}
                class="flex-1 rounded-lg border px-3 py-1.5 text-sm capitalize"
                classList={{
                  "border-blue-500 bg-blue-500/10 text-blue-600 dark:text-blue-400":
                    s()!.theme === t,
                  "border-black/10 dark:border-white/15": s()!.theme !== t,
                }}
              >
                {t}
              </button>
            ))}
          </div>
        </Field>

        <div class="space-y-1">
          <Toggle
            label="Launch at login"
            checked={s()!.launchAtLogin}
            onChange={(v) => patch({ launchAtLogin: v })}
          />
          <Toggle
            label="Auto-replace on Enter"
            checked={s()!.autoReplace}
            onChange={(v) => patch({ autoReplace: v })}
          />
          <Toggle
            label="Auto-copy result"
            checked={s()!.autoCopy}
            onChange={(v) => patch({ autoCopy: v })}
          />
          <Toggle
            label="Show notifications"
            checked={s()!.showNotifications}
            onChange={(v) => patch({ showNotifications: v })}
          />
        </div>

        <div class="flex items-center gap-3 pt-2">
          <button
            onClick={save}
            class="rounded-lg bg-blue-500 px-4 py-2 text-sm font-medium text-white hover:bg-blue-600"
          >
            Save
          </button>
          <Show when={saved()}>
            <span class="text-sm text-green-500">Saved ✓</span>
          </Show>
        </div>
      </div>
    </Show>
  );
};

const Field: Component<{ label: string; children: any }> = (props) => (
  <div>
    <label class="mb-1.5 block text-sm font-medium">{props.label}</label>
    {props.children}
  </div>
);

const Toggle: Component<{
  label: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}> = (props) => (
  <label class="flex cursor-pointer items-center justify-between py-2 text-sm">
    <span>{props.label}</span>
    <button
      type="button"
      onClick={() => props.onChange(!props.checked)}
      class="relative h-6 w-10 rounded-full transition-colors"
      classList={{
        "bg-blue-500": props.checked,
        "bg-black/15 dark:bg-white/20": !props.checked,
      }}
    >
      <span
        class="absolute top-0.5 h-5 w-5 rounded-full bg-white shadow transition-transform"
        classList={{ "translate-x-4.5 left-0.5": props.checked, "left-0.5": !props.checked }}
        style={props.checked ? { transform: "translateX(16px)" } : {}}
      />
    </button>
  </label>
);
