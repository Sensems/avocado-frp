# Avocado FRP

**Avocado FRP** is a cross-platform FRP (Fast Reverse Proxy) desktop ops console built with Tauri and Vue 3. It manages bundled `frpc` / `frps` sidecars, TOML configs, and process lifecycle from a single window.

---

## Why Avocado FRP?

FRP is powerful, but CLI-only workflows are awkward for day-to-day ops:

1. **Opaque config** — editing `toml` by hand is error-prone without validation feedback.
2. **Process babysitting** — starting/stopping `frpc` / `frps` without a supervisor is tedious.
3. **Unclear health** — hard to see phase, pending restart, and recent faults at a glance.

Avocado FRP provides a focused operations UI for those jobs.

---

## Features (implemented)

- **Ops console shell** — left nav (Overview / Client / Server / Logs / Settings), top status bar with frpc/frps phase, pending restart, and last fault. Default route: Overview. Minimum window size 1024×768.
- **Overview** — process cards, config issue summary, recent error lines, traffic summary when frpc local monitor is healthy (with empty states for disabled / stopped / port conflict / timeout / auth / not configured).
- **Client & Server** — Start / Stop / Save / Save & Restart; form ↔ source dual mode; mode switch never autosaves; Apply is explicit.
- **Source editor** — CodeMirror TOML with Validate → Preview → Apply (revision-checked; conflicts rejected).
- **Lossless config patches** — form saves apply minimal patches and keep unknown fields / comments where the backend supports them.
- **Logs** — filter by source / level / keyword, pause auto-scroll, clear UI buffer only (does not delete disk logs), copy visible lines, export via backend. Disk log deletion requires explicit confirmation.
- **Log rotation & retention** — process logs rotate when the active file exceeds the configured size; rotated history count is capped by policy (`maxFileBytes` / `maxRotatedFiles` in Settings, with bounds). Changing policy does not bulk-delete existing oversized files; only subsequent writes follow the new policy.
- **Diagnostics** — Run Diagnostics reports pass / warning / fail per check with suggested fix actions (sidecar presence, config validity, port conflicts, and related ops checks). Export a redacted diagnostics pack (no plaintext token / password / secret in the bundle).
- **Local monitor** — optional frpc admin/webServer prefs (default loopback `127.0.0.1:7400`); Overview traffic uses structured status empty states instead of a hard-coded remote URL.
- **Settings** — autostart, theme, locale, restore `.toml.bak` with confirmation; app version display; editable log rotation policy and local monitor prefs; **Check for updates** (download + confirm-before-install; optional silent check on launch).
- **Tray / shutdown** — closing can stay in tray; quit prepares shutdown so app-owned sidecars do not linger.
- **i18n** — zh / en.
- **Bundled binaries** — `frpc` / `frps` shipped with the app (no separate PATH setup). Expected FRP release: **0.67.0**.
- **CI / Release** — PR quality gates (`typecheck`, `build`, Rust fmt/clippy, version + sidecar prepare); Release prepares per-target sidecars and **fail-closes** without updater signing Secrets.

Not in scope for this product slice: Windows Authenticode, Apple Developer ID / notarization, or silent auto-install of updates.

---

## Usage

### 1. Install

Download a release build for your OS (local packages may also appear under `src-tauri/target/release/bundle/`).

**OS trust prompts:** this project does **not** ship Windows Authenticode or Apple notarization. Windows SmartScreen and macOS Gatekeeper may warn on first run. That is expected until you add platform code signing outside this repo’s updater signing.

**macOS Gatekeeper workaround** (unsigned / unnotarized builds):

```bash
sudo xattr -rd com.apple.quarantine "/Applications/Avocado FRP.app"
```

### 2. Configure

1. Open **Server** to set bind port / token (form or source), then **Apply** / **Save**.
2. Open **Client** to set server address and proxy rules, then **Apply** / **Save**.
3. Form ↔ source switches discard unapplied drafts only after confirm; they never write the file implicitly.

### 3. Start / stop

Use Overview or the Client/Server ops bar. The top bar shows phase and “restart pending” when config changed while a process is running.

### 4. Logs & diagnostics

1. **Logs** — clear screen only clears the UI buffer. Use delete-from-disk with confirmation to remove managed log files.
2. **Settings** — set max file size and rotated history count; enable local monitor (loopback by default) when you want Overview traffic.
3. **Diagnostics** — run checks from Settings (or the diagnostics panel), then optionally export a redacted pack for support.

### 5. Backup restore

Successful replaces keep `<name>.toml.bak`. Settings → restore asks for confirmation and respects revision conflicts.

### 6. Updates

1. **Settings → Check for updates** (or optional silent check on launch — never downloads/installs automatically).
2. Review version / notes, then **Install** and confirm.
3. Confirm stops app-owned `frpc` / `frps`, then downloads and installs.
4. Choose **Restart now** to relaunch into the new version.

---

## Development

Requirements: Node.js 18+, Rust 1.70+, pnpm.

```bash
git clone https://github.com/sensems/avocado-frp.git
cd avocado-frp
pnpm install
pnpm tauri dev
pnpm tauri build
```

### App version (SSOT)

Application version is defined in **`src-tauri/tauri.conf.json` → `version`**.  
`package.json` and `src-tauri/Cargo.toml` `[package].version` must match exactly.

```bash
pnpm check:versions
```

Fails (non-zero exit) when the three strings differ.

### Sidecar binaries (FRP 0.67.0)

Official FRP artifacts and SHA256 sums live in **`sidecar.manifest.json`** (pinned `frpVersion`: **0.67.0**).  
`SUPPORTED_FRP_VERSION` in the Rust sidecar adapter must stay aligned with that pin.

Before local/CI packaging, prepare or verify binaries for a target triple:

```bash
# host default, or:
pnpm prepare:sidecars --target x86_64-pc-windows-msvc
pnpm prepare:sidecars --all
```

Behavior: if `src-tauri/bin/frpc-<triple>[.exe]` / `frps-…` already match the manifest SHA256, download is skipped; otherwise download → verify → atomic place. SHA256 mismatch exits non-zero and does not keep unverified files.

Supported triples:

| Triple | Platform |
|--------|----------|
| `x86_64-pc-windows-msvc` | Windows x64 |
| `x86_64-unknown-linux-gnu` | Linux x64 |
| `aarch64-apple-darwin` | macOS Apple Silicon |
| `x86_64-apple-darwin` | macOS Intel |

### Useful checks

```bash
pnpm check:versions
pnpm prepare:sidecars --target x86_64-pc-windows-msvc
pnpm typecheck
pnpm build
cargo fmt --check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

PR CI is defined in `.github/workflows/ci.yml` (version check, sidecar prepare, frontend typecheck/build, Rust fmt/clippy).

### Updater signing boundary

| Item | Where |
|------|--------|
| **Public key** | In repo: `src-tauri/tauri.conf.json` → `plugins.updater.pubkey` (also mirrored as `src-tauri/keys/*.key.pub`) |
| **Private key** | **Never** in git. GitHub Secrets / controlled local env only. `src-tauri/keys/*.key` is gitignored. |
| **Endpoints** | GitHub Releases `latest.json` (see `plugins.updater.endpoints`) |
| **Platform code signing** | **Not** configured — no Authenticode / Apple notarization in this workflow |

Generate a keypair locally:

```bash
pnpm tauri signer generate -w src-tauri/keys/avocado-frp.key
```

Put the **public** key into `tauri.conf.json`. Store the private key only in Secrets (or a locked local path). Diagnostics packs and logs must never contain the private key.

After install, the app can relaunch via `@tauri-apps/plugin-process` (**Restart now**). Updates are never installed without an explicit confirm in Settings.

### Release CI secrets

Release workflow (`.github/workflows/release.yml`) is **fail-closed** on missing updater signing keys. Configure these repository Secrets before tagging `v*` or running `workflow_dispatch`:

| Secret | Required | Purpose |
|--------|----------|---------|
| `TAURI_SIGNING_PRIVATE_KEY` | **Yes** | Tauri updater minisign private key (contents or path). Never commit. |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | If key is encrypted | Password for the private key |

Each Release matrix job runs `pnpm check:versions`, `pnpm prepare:sidecars --target <triple>`, then builds with updater artifacts. Empty `TAURI_SIGNING_PRIVATE_KEY` → job exits `1` before `tauri-action`.

Release does **not** set up Windows Authenticode or Apple notarization; SmartScreen / Gatekeeper may still warn.

## License

MIT License
