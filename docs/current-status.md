# Current Status

Last updated: 2026-04-23

## Overview

`ai_usage_dashboard` is no longer just a planning repo.

Phase 11 complete in the working tree: threshold notifications and dynamic tray icon landed, and `npm run typecheck`, `cargo check`, `cargo clippy --all-targets -- -D warnings`, and `npm run build --workspace @ai-usage-dashboard/desktop` all pass.

Phase 13 is complete in the working tree: provider-specific connection guides, first-class issue states, and persisted Home compact mode are added on top of Phase 11.

Phase 12 is complete in the working tree: light/dark tokens are fully branched and brand colors are preserved across themes.

### Latest UI polish pass (2026-04-22)

Applied after visual verification on macOS:

- Tray icon rendered pure white regardless of template mode; template mode disabled so menubar logos stay high-contrast on both light/dark menubars.
- Tray target default flipped from `max` to `last-viewed`; migration in `BrowserSettingsStore.load` rewrites persisted `"max"` values so existing installs pick up the new default.
- `homeCompactView` default flipped to `true`; Home now opens in compact mode by default.
- Keychain reads cached in-memory via `shared::read_keychain_cached` so repeated refreshes no longer trigger repeated macOS keychain prompts. Save/clear operations prime/invalidate the cache.
- Removed the "Menubar Icon" row from Settings (the option had no effect after tray-renderer unification).
- Notifications thresholds split into two separate settings rows (`1차 알림` / `2차 알림`) so the layout no longer overflows.
- Provider cards respect `displayMode` in compact mode: `used` vs `left` now flips the headline percentage and label.
- Unconfigured providers surface an inline "연결 방법" affordance that opens a provider-specific connection guide modal.
- All user-facing copy translated to Korean (cards, tabs, settings, modals, relative time formatter, tray menu).
- `cursor-pointer` applied globally to buttons plus the tab/segmented/sidenav primitives; `button:disabled` falls back to `not-allowed`.
- Legacy "demo" credential placeholders purged from browser storage on boot.

It now has:

- a typed workspace split into `core`, `platform`, `providers`, and `apps/desktop`
- a runnable React/Vite desktop shell
- a Tauri v2 native shell
- live Codex, Claude, Copilot, and OpenRouter desktop integrations
- a menu bar app flow on macOS

## Repository Structure

### Root

- `package.json`
- `tsconfig.json`
- `tsconfig.base.json`
- `.gitignore`

### Packages

- `packages/core`
  shared domain models, settings, snapshots, refresh orchestration
- `packages/platform`
  platform contracts for credential store and settings store
- `packages/providers`
  provider adapters for `codex`, `claude`, `copilot`, and `openrouter`

### App

- `apps/desktop`
  React/Vite frontend shell
- `apps/desktop/src-tauri`
  Tauri v2 native runtime

## Live Integration Status

Phase 14 is complete in the working tree: native credentials now resolve through a shared `CredentialSource` registry with startup warm-up and Codex/Claude OAuth refresh persistence.

Phase 15 is complete in the working tree: light-mode semantic tokens are finished, `ko`/`en` localization is bundled through desktop i18n, interactive controls have broader accessibility coverage, and tray positioning/menu labels are polished.

Phase 16 is complete in the working tree: provider schema/backoff resilience, shell-launched CLI connect flow, Home simplification, and reset-timer guidance are now wired through the desktop app.

Phase 17 is complete in the working tree: Settings trims, implicit platform detection, switch contrast, and Home/Detail connect CTA placement are landed.

Phase 18 is complete in the working tree: the tray first-paint now uses a Rust-side white mask, Claude diagnostics are env-var gated, and the SideNav/Home/footer polish is landed.

### Providers

Implemented in the desktop shell:

- Codex
  - auth source: `~/.codex/auth.json`
  - remote usage source: `https://chatgpt.com/backend-api/wham/usage`
  - local token usage source: `~/.codex/sessions/**/*.jsonl`
- Claude
  - auth source: `~/.claude/.credentials.json`, `CLAUDE_CONFIG_DIR`, or native keychain
  - remote usage source: `https://api.anthropic.com/api/oauth/usage`
- Copilot
  - auth source: native keychain, `gh` keychain entry, or app state file
  - remote usage source: `https://api.github.com/copilot_internal/user`
- OpenRouter
  - auth source: native keychain or `OPENROUTER_API_KEY`
  - remote usage source: `https://openrouter.ai/api/v1/key`

Current metric payloads cover shared progress, text, and badge lines across the Tauri commands and frontend adapters.

## Native Runtime Status

### Tauri v2

Added and validated.

Key files:

- [apps/desktop/src-tauri/Cargo.toml](../apps/desktop/src-tauri/Cargo.toml)
- [apps/desktop/src-tauri/src/lib.rs](../apps/desktop/src-tauri/src/lib.rs)
- [apps/desktop/src-tauri/tauri.conf.json](../apps/desktop/src-tauri/tauri.conf.json)

Implemented native commands:

- `get_codex_usage`
- `get_claude_usage`
- `get_copilot_usage`
- `get_openrouter_usage`
- `save_openrouter_key`
- `clear_openrouter_key`
- `has_openrouter_key`
- `detect_platform`

Validation completed:

- `cargo check` passed
- `cargo clippy --all-targets -- -D warnings` passed
- `npm run typecheck` passed
- `npm run build --workspace @ai-usage-dashboard/desktop` passed
- `tauri build` passed
- release binary generated successfully

Release binary path:

- [ai_usage_dashboard](../apps/desktop/src-tauri/target/release/ai_usage_dashboard)

## macOS Menu Bar Behavior

Implemented.

Current behavior:

- app uses `Accessory` activation policy on macOS
- app creates a tray/menu bar icon
- app starts hidden
- clicking the tray icon toggles window show/hide
- closing the window hides it instead of exiting
- tray menu includes:
  - `Show Dashboard`
  - `Go to Settings`
  - `Quit`

Relevant files:

- [apps/desktop/src-tauri/src/lib.rs](../apps/desktop/src-tauri/src/lib.rs)
- [apps/desktop/src-tauri/icons/tray-icon.png](../apps/desktop/src-tauri/icons/tray-icon.png)

## Frontend UI Status

### Base Shell

Implemented in:

- [apps/desktop/src/App.tsx](../apps/desktop/src/App.tsx)
- [apps/desktop/src/styles.css](../apps/desktop/src/styles.css)

Current UI model:

- left sidebar nav
- right content panel
- `home`, `provider detail`, `settings` views
- footer actions

### Design Direction

The UI has been iterated toward the original `openusage` GitHub app:

- dark compact panel layout
- narrow left sidebar
- provider card layout closer to the original app
- settings layout aligned with the original sidebar-driven flow
- muted footer + compact card density

### Current Compact Panel State

The current UI is now tuned for a compact menu bar panel:

- panel resized down to a compact menu bar panel size
- home view filtered so only live-connected services are shown
- additional visual polish for compact cards and section intros

Primary files:

- [apps/desktop/src/App.tsx](../apps/desktop/src/App.tsx)
- [apps/desktop/src/styles.css](../apps/desktop/src/styles.css)
- [apps/desktop/src-tauri/tauri.conf.json](../apps/desktop/src-tauri/tauri.conf.json)

## Validation Performed

Completed during development:

- `npm install`
- `npm run typecheck`
- `npm run build`
- local `/api/codex/usage` response verification
- `cargo check`
- `tauri build`
- `npm run dev:tauri`

Observed live Codex response during testing included:

- plan: `Pro 5x`
- session usage percent
- weekly usage percent
- token totals from local Codex sessions

## Important Notes

### Stability

The Codex usage path is practical but not based on a public stable OpenAI API contract.

That means:

- upstream auth shape may change
- backend usage response shape may change
- local Codex session log structure may change

### Current Scope

This is now a working foundation for a real desktop app, but not a fully productized release yet.

Still missing or incomplete:

- secure cross-platform credential storage abstraction in production form (macOS keychain cache mitigates prompt spam but cache is process-local only)
- polished tray positioning/panel behavior like the original macOS app (basic clamp-to-work-area landed; still want hover preview + drag anchor behavior)
- polished light-mode tokens beyond the new functional `html.light`/`html.dark` setting toggle
- reliable DMG bundling in sandboxed / AppleScript-restricted environments
- Windows cross-target validation on a host with the Windows Rust target installed
- localization infrastructure: copy is hard-coded Korean right now; a future pass should extract strings into a locale bundle

## Commit History Summary

Recent commits:

- `014a6c9` docs: add initial cross-platform architecture and roadmap
- `b92c9eb` build: scaffold workspace for desktop core and providers
- `c5a7965` build: ignore TypeScript build metadata
- `ebf43bc` feat: add runnable desktop shell ui
- `7258384` feat: add live codex usage integration
- `119f409` feat: add tauri codex desktop shell
- `94432b2` feat: align desktop ui with openusage layout
- `63ba613` feat: add menubar tray behavior

## Recommended Next Steps

1. Refine tray/panel sizing and open/close behavior until it feels native.
2. Harden credential storage and auth refresh handling for each live provider.
3. Validate Windows runtime behavior on a host with the Windows Rust target installed.
4. Make DMG packaging reliable on hosts that restrict Finder AppleScript automation.
5. Package the macOS app more cleanly for repeatable local installs.
