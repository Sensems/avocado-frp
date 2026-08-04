## Learned User Preferences

- Prefers Chinese for planning and product discussion replies.
- Prefers phased work-package flow: confirm/write the plan under `docs/superpowers/`, then execute one package at a time when asked.
- When offered execution style, prefers Subagent-Driven Development over long inline execution.
- For later work packages (from WP2 onward), prefers skipping new unit/component tests; verify with `pnpm typecheck`, `pnpm build`, and manual acceptance instead.
- Does not want Git commits unless explicitly authorized in the execution phase.

## Learned Workspace Facts

- Product is Avocado FRP: a Tauri 2 + Vue 3 desktop ops console for bundled `frpc` / `frps` sidecars, TOML config, and process lifecycle.
- Full-system optimization is staged under `docs/superpowers/` — design spec plus WP1 safety-baseline, WP2 process-config-services, WP3 ops-console, with WP4 logs/diagnostics and WP5 CI/updater still later.
- Windows is the full local runtime acceptance target; macOS/Linux are CI build and smoke only for these packages.
- Frontend Tauri IPC must go only through `src/services/tauriClient.ts` (`invoke` / `listen`).
- Config writes use `toml_edit::DocumentMut` with revision checks, minimal patches, `.toml.bak` backups, and atomic replace; unknown TOML fields must be preserved.
- Process lifecycle is supervised via typed phase state (not a bare boolean), with unified shutdown so app-owned sidecars do not linger.
- Do not modify, delete, or stage unrelated untracked `.codegraph/` or `.cursor/` paths during work-package execution.
- Ops console UI targets a professional ops look (neutral surfaces, clear status colors); form ↔ source mode switches must not implicitly save.
