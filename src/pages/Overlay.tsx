import { type Component, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { LoadingSpinner } from "../components/LoadingSpinner";
import { Toolbar } from "../components/Toolbar";
import * as api from "../lib/api";
import type { StreamEvent } from "../lib/types";

type Status = "loading" | "streaming" | "done" | "error";

const LANGUAGES = [
  { code: "en", label: "English" },
  { code: "id", label: "Indonesian" },
  { code: "es", label: "Spanish" },
  { code: "ja", label: "Japanese" },
  { code: "zh-CN", label: "Chinese" },
  { code: "fr", label: "French" },
  { code: "de", label: "German" },
  { code: "ko", label: "Korean" },
  { code: "pt", label: "Portuguese" },
  { code: "ru", label: "Russian" },
  { code: "it", label: "Italian" },
  { code: "ar", label: "Arabic" },
  { code: "hi", label: "Hindi" },
  { code: "th", label: "Thai" },
  { code: "vi", label: "Vietnamese" },
];

/**
 * The floating popup. Listens to `virex://stream` events from the backend,
 * renders the rewritten text as it arrives, and exposes Replace/Copy/Retry/Translate.
 */
export const Overlay: Component = () => {
  const [text, setText] = createSignal("");
  const [status, setStatus] = createSignal<Status>("loading");
  const [error, setError] = createSignal("");
  const [targetLang, setTargetLang] = createSignal("en");

  const handleEvent = (e: StreamEvent) => {
    switch (e.kind) {
      case "start":
        setText("");
        setError("");
        setStatus("loading");
        break;
      case "delta":
        setStatus("streaming");
        setText((t) => t + e.text);
        break;
      case "done":
        setText(e.full);
        setStatus("done");
        break;
      case "error":
        setError(e.message);
        setStatus("error");
        break;
    }
  };

  const doReplace = () => {
    if (status() !== "done" && status() !== "streaming") return;
    void api.replaceSelection(text());
  };
  const doCopy = () => void api.copyToClipboard(text());
  const doRetry = () => void api.retry();
  const doTranslate = (lang?: string) => {
    const selected = lang || targetLang();
    void api.translateSelection(selected);
  };
  const doClose = () => void api.closeOverlay();

  const onKey = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      e.preventDefault();
      doClose();
    } else if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      doReplace();
    }
  };

  onMount(async () => {
    const settings = await api.getSettings();
    if (settings?.targetLang) {
      setTargetLang(settings.targetLang);
    }
    const unlisten = await api.onStream(handleEvent);
    window.addEventListener("keydown", onKey);
    onCleanup(() => {
      unlisten();
      window.removeEventListener("keydown", onKey);
    });
  });

  return (
    <div class="animate-pop-in p-1.5">
      <div class="vx-card rounded-2xl p-3">
        <div class="mb-2 flex items-center gap-2 px-0.5">
          <div class="h-2 w-2 rounded-full bg-blue-500" />
          <span class="text-[11px] font-semibold uppercase tracking-wide text-black/40 dark:text-white/40">
            Virex
          </span>
          <div class="flex-1" />
          <div class="flex items-center gap-1.5">
            <span class="text-[11px] text-black/40 dark:text-white/40">Translate to:</span>
            <select
              value={targetLang()}
              onChange={(e) => {
                const newLang = e.currentTarget.value;
                setTargetLang(newLang);
              }}
              class="rounded-md border border-black/10 bg-black/5 px-1.5 py-0.5 text-[11px] font-medium text-black/80 outline-none hover:bg-black/10 dark:border-white/15 dark:bg-white/10 dark:text-white/80 dark:hover:bg-white/20"
            >
              <For each={LANGUAGES}>
                {(l) => <option value={l.code}>{l.label}</option>}
              </For>
            </select>
          </div>
          <Show when={status() === "streaming"}>
            <span class="text-[11px] text-black/30 dark:text-white/30">
              writing…
            </span>
          </Show>
        </div>

        <div class="vx-scroll max-h-64 overflow-y-auto px-0.5">
          <Show when={status() === "error"}>
            <p class="text-[13px] leading-relaxed text-red-500">{error()}</p>
          </Show>

          <Show when={status() === "loading"}>
            <LoadingSpinner />
          </Show>

          <Show when={status() === "streaming" || status() === "done"}>
            <p class="vx-selectable whitespace-pre-wrap text-[14px] leading-relaxed text-black/90 dark:text-white/90">
              {text()}
              <Show when={status() === "streaming"}>
                <span class="ml-0.5 inline-block h-4 w-[2px] translate-y-0.5 animate-pulse bg-blue-500 align-middle" />
              </Show>
            </p>
          </Show>
        </div>

        <div class="mt-3 flex items-center justify-between border-t border-black/5 pt-2.5 dark:border-white/10">
          <Toolbar
            actions={[
              {
                label: "Replace",
                hint: "↵",
                primary: true,
                onClick: doReplace,
                disabled: status() === "loading" || status() === "error",
              },
              { label: "Copy", onClick: doCopy, disabled: !text() },
              { label: "Translate", onClick: () => doTranslate() },
              { label: "Retry", onClick: doRetry },
            ]}
          />
          <button
            onClick={doClose}
            class="rounded-lg px-2 py-1.5 text-[13px] text-black/40 hover:bg-black/5 dark:text-white/40 dark:hover:bg-white/10"
          >
            Esc
          </button>
        </div>
      </div>
    </div>
  );
};
