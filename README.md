# Virex

> Rewrite anything, right where you type.

A tiny native macOS writing assistant that lives in your menu bar. Highlight text
in **any** app, press **⌘⇧1**, and Virex streams a cleaner version into a small
floating window. Press **Enter** to replace your selection in place.

Built with Rust + Tauri v2 + SolidJS. No dock icon, no Electron, no subscription.

**[Download the latest release](https://github.com/phcodesage/Virex/releases/latest)** ·
**[virex-eta.vercel.app](https://virex-eta.vercel.app)**

---

## Install

1. Download `Virex_<version>_aarch64.dmg` from
   [Releases](https://github.com/phcodesage/Virex/releases/latest)
   (Apple Silicon, macOS 12+).
2. Open the DMG and drag **Virex** to Applications.
3. Virex isn't signed with a paid Apple Developer certificate, so macOS blocks
   downloaded copies. Clear the quarantine flag once:

   ```bash
   xattr -cr /Applications/Virex.app
   ```

   > On macOS 15 and later, right-click → Open no longer bypasses Gatekeeper,
   > so this command is the reliable path until the app is notarized.
4. Launch Virex and open **Settings** from the menu-bar icon to finish setup.

## Setup

Two things are required, both in **Settings**:

| | Why |
|---|---|
| **Accessibility** access | Lets Virex read your selection and paste the replacement. Virex triggers the system prompt and detects the moment you grant it. |
| **DeepSeek API key** | Virex is bring-your-own-key. Your text goes straight to DeepSeek — there is no Virex server. The key is stored in the macOS **Keychain**, never on disk. |

Get a key at [platform.deepseek.com](https://platform.deepseek.com).

## Use it

```
Highlight text  →  ⌘⇧1  →  capture selection (⌘C)  →  DeepSeek (streaming)
                          →  overlay updates live   →  Enter → paste (⌘V)
```

| Key | Action |
|---|---|
| `⌘⇧1` | Rewrite the current selection (configurable) |
| `Enter` | Replace the selection with the rewrite |
| `Esc` | Close the overlay |

The overlay also has **Replace**, **Copy**, and **Retry** buttons, and can be
dragged anywhere by its header.

Because Virex drives the system copy/paste shortcuts rather than per-app text
APIs, it works consistently across Chrome, Safari, Slack, Discord, WhatsApp,
VS Code, Notion, Apple Notes, and Mail.

### Menu-bar menu

Open Settings · Pause · Resume · Check for Updates · Quit.
**Check for Updates** compares your version against the latest GitHub release.

## Configuration

Settings exposes only what you need to get running (Accessibility + API key).
Everything else lives in:

```
~/Library/Application Support/com.virex.app/settings.toml
```

| Key | Default | Notes |
|---|---|---|
| `model` | `deepseek-chat` | Any OpenAI-compatible model ID string. |
| `temperature` | `0.2` | Low by default so rewrites are consistent, not creative. |
| `system_prompt` | paraphrasing prompt | Rewrites for grammar, clarity, and awkward phrasing. |
| `shortcut` | `Super+Shift+1` | Tauri accelerator syntax (`Super` = ⌘ on macOS). |
| `launch_at_login`, `auto_copy`, `show_notifications`, `theme` | | |

## Build from source

Requirements: Rust (stable), Node 18+, pnpm.

```bash
git clone https://github.com/phcodesage/Virex.git
cd Virex
pnpm install

pnpm tauri dev      # run in development
pnpm tauri build    # produce a .app / .dmg
```

For development you can put a key in `.env` (`cp .env.example .env`) and it is
moved into the Keychain on first launch. `.env` is gitignored.

## Privacy

- Your text is sent **only** when you trigger a rewrite, and only to DeepSeek
  using **your own** API key. Nothing passes through a Virex server.
- The API key lives in the macOS Keychain (service `com.virex.app`).
- Prompts, selected text, rewrite output, and keys are **never** logged.

## Architecture

`src-tauri/src/`

| File | Responsibility |
|---|---|
| `lib.rs` | App bootstrap, plugins, setup wiring. |
| `pipeline.rs` | The capture → stream → overlay flow. |
| `hotkeys.rs` | Global shortcut registration. |
| `selection.rs` · `replace.rs` · `input.rs` | Clipboard + ⌘C/⌘V capture & replace. |
| `axselect.rs` | Accessibility lookup of the selection's on-screen rect, so the overlay can anchor to your text. |
| `frontmost.rs` | Tracks/reactivates the source app so paste lands correctly. |
| `deepseek.rs` | Streaming chat-completions client. |
| `overlay.rs` · `window.rs` | Floating overlay + Settings windows. |
| `accessibility.rs` | Permission detection & prompting. |
| `keychain.rs` | Secure API key storage. |
| `settings.rs` · `config.rs` · `state.rs` | Config model, paths, shared state. |
| `tray.rs` | Menu-bar icon and menu. |
| `events.rs` | Streaming events to the webview. |
| `updater.rs` | GitHub-release update check. |
| `notify.rs` · `logging.rs` | Notifications, logging. |

Frontend (`src/`): SolidJS + Tailwind. `pages/Overlay.tsx` is the floating
window, `pages/Settings.tsx` is the settings window — the same webview bundle
switched by window label. `landing/` holds the static marketing site.

## Known limitations

- **Apple Silicon only** builds are published; build from source for Intel.
- **Not notarized.** Gatekeeper rejects the app until it's signed with a paid
  Apple Developer ID, hence the `xattr` step above.
- Anchoring the overlay to the selected text relies on macOS Accessibility
  (`AXBoundsForRange`), which many browsers don't expose — Virex falls back to
  positioning near the mouse cursor there.

## Support

Virex is free and open source. If it saves you an embarrassing typo,
you can [buy me a coffee](https://ko-fi.com/phcodesage).

## License

[MIT](LICENSE) © phcodesage
