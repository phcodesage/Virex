/* @refresh reload */
import { render } from "solid-js/web";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Overlay } from "./pages/Overlay";
import { Settings } from "./pages/Settings";
import * as api from "./lib/api";
import "./styles.css";

// The Rust side creates two windows with labels "overlay" and "settings".
// We render the matching page based on the current window label.
const label = getCurrentWindow().label;

// Apply the saved theme to <html data-theme>.
async function applyTheme() {
  try {
    const { theme } = await api.getSettings();
    const dark =
      theme === "dark" ||
      (theme === "system" &&
        window.matchMedia("(prefers-color-scheme: dark)").matches);
    document.documentElement.dataset.theme = dark ? "dark" : "light";
  } catch {
    /* backend not ready yet — default to system */
  }
}
void applyTheme();

const root = document.getElementById("root")!;
render(() => (label === "settings" ? <Settings /> : <Overlay />), root);
