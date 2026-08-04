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
- **Settings** — autostart, theme, locale, restore `.toml.bak` with confirmation; app version display; editable log rotation policy and local monitor prefs. Updater controls remain disabled placeholders labeled WP5.
- **Tray / shutdown** — closing can stay in tray; quit prepares shutdown so app-owned sidecars do not linger.
- **i18n** — zh / en.
- **Bundled binaries** — `frpc` / `frps` shipped with the app (no separate PATH setup).

Not claimed in this package: Tauri updater, signed releases, sidecar remote download / SHA256 manifest, or PR CI quality gates (WP5).

---

## Usage

### 1. Install

Download a release build for your OS (local packages may also appear under `src-tauri/target/release/bundle/`).

**macOS unsigned note:** if Gatekeeper blocks the app:

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

Useful checks:

```bash
pnpm typecheck
pnpm build
cargo check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

## License

MIT License
