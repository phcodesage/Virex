import type { Component } from "solid-js";
import { For } from "solid-js";

interface Action {
  label: string;
  hint: string;
  onClick: () => void;
  primary?: boolean;
  disabled?: boolean;
}

/** The bottom action row of the floating window. */
export const Toolbar: Component<{ actions: Action[] }> = (props) => {
  return (
    <div class="flex items-center gap-1.5">
      <For each={props.actions}>
        {(a) => (
          <button
            disabled={a.disabled}
            onClick={a.onClick}
            class="group flex items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-[13px] font-medium transition-colors disabled:opacity-40"
            classList={{
              "bg-blue-500 text-white hover:bg-blue-600 active:bg-blue-700":
                a.primary,
              "text-black/70 hover:bg-black/5 dark:text-white/70 dark:hover:bg-white/10":
                !a.primary,
            }}
          >
            <span>{a.label}</span>
            <kbd
              class="rounded px-1 text-[10px] leading-4 opacity-60"
              classList={{
                "bg-white/20": a.primary,
                "bg-black/10 dark:bg-white/10": !a.primary,
              }}
            >
              {a.hint}
            </kbd>
          </button>
        )}
      </For>
    </div>
  );
};
