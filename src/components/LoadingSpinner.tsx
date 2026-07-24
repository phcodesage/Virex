import type { Component } from "solid-js";

/** A minimal shimmering placeholder shown while the first tokens arrive. */
export const LoadingSpinner: Component = () => {
  return (
    <div class="space-y-2 py-1">
      {[100, 92, 76].map((w) => (
        <div
          class="relative h-3 overflow-hidden rounded bg-black/5 dark:bg-white/10"
          style={{ width: `${w}%` }}
        >
          <div class="absolute inset-0 -translate-x-full animate-shimmer bg-gradient-to-r from-transparent via-black/10 to-transparent dark:via-white/10" />
        </div>
      ))}
    </div>
  );
};
