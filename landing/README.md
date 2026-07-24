# Virex landing page

A single self-contained static page (`index.html`) — no build step, no dependencies.

## Deploy

### Netlify
The repo root already has `netlify.toml` pointing the publish directory at `landing/`.
Just "Add new site → Import from Git" and it deploys as-is.

### Vercel
Vercel needs the subdirectory set manually:

1. Import the repo.
2. **Root Directory** → `landing`.
3. **Framework Preset** → `Other`.
4. Leave Build Command empty and Output Directory empty (it serves the folder as static).
5. Deploy.

## Before you ship, replace these placeholders

All links currently point at sensible defaults — confirm they're right:

- **Download** → `https://github.com/phcodesage/Virex/releases/latest`
- **GitHub** → `https://github.com/phcodesage/Virex`
- **Ko-fi** (tips and Pro) → `https://ko-fi.com/phcodesage` — create a
  **$10/month membership tier** there, and wire up the webhook described in
  `worker/README.md` so payments issue licence keys.

Search `index.html` for those URLs to update them. Pricing lives in the
`#pricing` section; the Pro price appears once, in the `.price` element.
