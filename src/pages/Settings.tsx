import { getVersion } from "@tauri-apps/api/app";
import { type Component, createSignal, onCleanup, onMount, Show } from "solid-js";
import * as api from "../lib/api";
import type { PlanInfo } from "../lib/api";

const UPGRADE_URL = "https://virex-eta.vercel.app/#pricing";

/**
 * Setup window. Virex needs one permission to work; everything else here is
 * plan status. There's no API key to enter — the Virex API proxy holds it.
 */
export const Settings: Component = () => {
  const [trusted, setTrusted] = createSignal(true);
  const [version, setVersion] = createSignal("");
  const [plan, setPlan] = createSignal<PlanInfo>();
  const [planError, setPlanError] = createSignal("");
  const [license, setLicenseInput] = createSignal("");
  const [savingLicense, setSavingLicense] = createSignal(false);
  const [licenseError, setLicenseError] = createSignal("");
  const [showLicense, setShowLicense] = createSignal(false);
  let poll: number | undefined;

  const refreshAccess = async () => {
    const ok = await api.accessibilityTrusted();
    setTrusted(ok);
    return ok;
  };

  const refreshPlan = async () => {
    try {
      setPlan(await api.getPlan());
      setPlanError("");
    } catch (e) {
      setPlanError(String(e));
    }
  };

  // Fire the native macOS prompt, then poll so the UI flips to "granted" the
  // instant the toggle is switched on.
  const requestAccess = async () => {
    await api.requestAccessibility();
    if (poll === undefined) {
      poll = window.setInterval(async () => {
        if (await refreshAccess()) {
          window.clearInterval(poll);
          poll = undefined;
        }
      }, 1000);
    }
  };

  const saveLicense = async () => {
    setSavingLicense(true);
    setLicenseError("");
    try {
      const result = await api.setLicense(license().trim());
      setPlan(result);
      if (result.plan === "pro") {
        setLicenseInput("");
        setShowLicense(false);
      } else if (result.seatLimited) {
        setLicenseError(
          `That key is already in use on ${result.maxSeats} devices. Free up a device or contact support.`,
        );
      } else if (license().trim()) {
        setLicenseError("That key isn't active. Check it and try again.");
      }
    } catch (e) {
      setLicenseError(String(e));
    } finally {
      setSavingLicense(false);
    }
  };

  onMount(async () => {
    getVersion().then(setVersion).catch(() => {});
    void refreshPlan();
    const ok = await refreshAccess();
    if (!ok) void requestAccess();
  });

  onCleanup(() => {
    if (poll !== undefined) window.clearInterval(poll);
  });

  const used = () => plan()?.used ?? 0;
  const limit = () => plan()?.limit ?? 0;
  const pct = () => (limit() ? Math.min(100, (used() / limit()) * 100) : 0);

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

      {/* Accessibility permission — the only thing required to work */}
      <Show
        when={!trusted()}
        fallback={
          <div class="rounded-xl border border-green-500/30 bg-green-500/10 p-4 text-sm">
            <p class="font-medium text-green-600 dark:text-green-400">
              Accessibility granted ✓
            </p>
            <p class="mt-1 opacity-70">
              Select text anywhere and press your shortcut.
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

      {/* Plan + usage */}
      <div class="rounded-xl border border-black/10 p-4 dark:border-white/15">
        <Show
          when={plan()}
          fallback={
            <p class="text-sm opacity-50">
              {planError() ? "Couldn't reach the Virex service." : "Checking your plan…"}
            </p>
          }
        >
          <Show
            when={plan()!.plan === "pro"}
            fallback={
              <>
                <div class="flex items-baseline justify-between">
                  <span class="text-sm font-medium">Free plan</span>
                  <span class="text-xs opacity-60">
                    {used()} of {limit()} today
                  </span>
                </div>
                <div class="mt-2 h-1.5 w-full overflow-hidden rounded-full bg-black/10 dark:bg-white/15">
                  <div
                    class="h-full rounded-full bg-blue-500 transition-[width]"
                    style={{ width: `${pct()}%` }}
                  />
                </div>
                <p class="mt-2 text-xs opacity-50">
                  {plan()!.remaining === 0
                    ? "You've used today's rewrites. They reset tomorrow."
                    : `${plan()!.remaining} rewrites left today. Resets daily.`}
                </p>
                <Show when={plan()!.seatLimited}>
                  <p class="mt-2 rounded-lg bg-amber-500/10 px-2.5 py-2 text-xs text-amber-600 dark:text-amber-400">
                    Your Pro key is already active on {plan()!.maxSeats} devices,
                    so this Mac is on the free plan. Deactivate another device or
                    get in touch and we'll free a seat.
                  </p>
                </Show>
                <div class="mt-3 flex items-center gap-2">
                  <button
                    onClick={() => api.openUrl(UPGRADE_URL)}
                    class="rounded-lg bg-blue-500 px-3 py-1.5 text-xs font-medium text-white hover:bg-blue-600"
                  >
                    Upgrade to Pro
                  </button>
                  <button
                    onClick={() => setShowLicense((v) => !v)}
                    class="rounded-lg px-3 py-1.5 text-xs font-medium text-black/60 hover:bg-black/5 dark:text-white/60 dark:hover:bg-white/10"
                  >
                    I have a key
                  </button>
                </div>
              </>
            }
          >
            <div class="flex items-baseline justify-between">
              <span class="text-sm font-medium text-blue-600 dark:text-blue-400">
                Pro — unlimited ✓
              </span>
              <span class="text-xs opacity-60">{used()} today</span>
            </div>
            <p class="mt-1 text-xs opacity-50">
              Thanks for supporting Virex.
              <Show when={plan()!.maxSeats > 0}>
                {" "}This key is on {plan()!.seatsUsed} of {plan()!.maxSeats} devices.
              </Show>
            </p>
          </Show>
        </Show>

        <Show when={showLicense() && plan()?.plan !== "pro"}>
          <div class="mt-3 space-y-2 border-t border-black/5 pt-3 dark:border-white/10">
            <input
              value={license()}
              onInput={(e) => setLicenseInput(e.currentTarget.value)}
              onKeyDown={(e) => e.key === "Enter" && saveLicense()}
              placeholder="VIREX-XXXX-XXXX"
              class="vx-input"
              autocomplete="off"
              spellcheck={false}
            />
            <div class="flex items-center gap-2">
              <button
                onClick={saveLicense}
                disabled={!license().trim() || savingLicense()}
                class="rounded-lg bg-blue-500 px-3 py-1.5 text-xs font-medium text-white hover:bg-blue-600 disabled:cursor-not-allowed disabled:opacity-40"
              >
                {savingLicense() ? "Checking…" : "Activate"}
              </button>
              <Show when={licenseError()}>
                <span class="text-xs text-red-500">{licenseError()}</span>
              </Show>
            </div>
          </div>
        </Show>
      </div>
    </div>
  );
};
