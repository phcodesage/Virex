# Virex

A native macOS AI writing assistant. Highlight text in **any** app, press
**⌘⇧P**, and Virex streams an improved version from DeepSeek into a tiny
floating window. Press **Enter** to replace the selection in place.

Built with Rust + Tauri v2 + SolidJS. Lives in the menu bar (no dock icon).

## How it works

```
Highlight text  →  ⌘⇧P  →  capture selection (⌘C)  →  DeepSeek (streaming)
                          →  floating overlay updates live  →  Enter → paste (⌘V)
```

Because it drives the system copy/paste shortcuts (rather than per-app
Accessibility text APIs), it works consistently across Chrome, Safari, Slack,
Discord, VS Code, Notion, Apple Notes, Mail, and more.

## Setup

Requirements: Rust (stable), Node 18+, pnpm.

```bash
cd virex
pnpm install

# Add your DeepSeek key (moved into the Keychain on first launch):
cp .env.example .env
#   then edit .env and paste your key, OR set it later in Settings.

pnpm tauri dev      # run in development
pnpm tauri build    # produce a .app / .dmg
```

If you don't have the Tauri CLI globally, `pnpm tauri …` uses the local one
from `devDependencies`.

### Permissions

On first trigger, macOS will ask for **Accessibility** access (needed to
synthesize ⌘C / ⌘V into other apps). Virex opens the right System Settings pane
for you. Grant it, then press ⌘⇧P again.

## Configuration

Open **Settings** from the menu-bar icon:

| Setting | Notes |
|---|---|
| API Key | Stored in the macOS **Keychain**, never on disk. |
| Model | Default `deepseek-chat`. See note below re: `deepseek-v4-flash`. |
| Temperature | 0.0–1.5 |
| System Prompt | The rewrite instructions. |
| Global Shortcut | Tauri accelerator syntax, e.g. `CmdOrCtrl+Shift+P`. |
| Launch at Login · Auto-Replace · Auto-Copy · Notifications · Theme | |

Settings persist to `~/Library/Application Support/com.virex.app/settings.toml`.

> **Model note.** DeepSeek's documented model IDs are `deepseek-chat` and
> `deepseek-reasoner`. The default is `deepseek-chat`. If `deepseek-v4-flash`
> is available on your account, just pick it in Settings — the model is a plain
> string sent to the OpenAI-compatible endpoint, so no code change is needed.

## Overlay shortcuts

`Enter` replace · `Esc` close · `⌘C` copy · `⌘R` retry

## Security

- API key lives in the macOS Keychain (service `com.virex.app`).
- Prompts, selected text, rewrite output, and keys are **never** logged.

## Architecture (`src-tauri/src/`)

| File | Responsibility |
|---|---|
| `lib.rs` | App bootstrap, plugins, setup wiring. |
| `pipeline.rs` | The capture → stream → overlay flow. |
| `hotkeys.rs` | Global shortcut registration. |
| `selection.rs` / `replace.rs` / `input.rs` | Clipboard + ⌘C/⌘V capture & replace. |
| `deepseek.rs` | Streaming chat-completions client. |
| `overlay.rs` / `window.rs` | Floating overlay + Settings windows. |
| `accessibility.rs` | Permission detection & prompting. |
| `keychain.rs` | Secure API key storage. |
| `settings.rs` / `config.rs` / `state.rs` | Config model, paths, shared state. |
| `tray.rs` | Menu-bar icon and menu. |
| `events.rs` | Streaming events to the webview. |
| `notify.rs` · `updater.rs` · `logging.rs` | Notifications, updates, logging. |

Frontend (`src/`): SolidJS + Tailwind. `pages/Overlay.tsx` is the floating
window; `pages/Settings.tsx` is the settings window; both are the same webview
bundle switched by window label.
