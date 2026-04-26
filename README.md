# AI Usage Dashboard

Cross-platform AI usage dashboard for macOS and Windows.

## Goal

Build a desktop app that shows AI subscription and usage data in one place without inheriting the full risk profile of an existing macOS-only fork.

The product starts with a clean architecture:

- shared provider core
- platform-specific desktop shell
- explicit credential access boundaries
- telemetry off by default
- local API off by default

## Product Scope

### Phase 1

- macOS desktop app
- tray access and compact dashboard window
- manual refresh
- local encrypted settings
- provider support for:
  - Claude
  - Codex
  - Cursor
  - Copilot

### Phase 2

- Windows desktop app
- Windows tray integration
- WebView2 shell
- Windows credential storage support
- shared provider engine reused from macOS build

## Non-Goals

- browser app first
- uncontrolled plugin execution
- broad filesystem access for providers
- telemetry by default
- public local HTTP API by default

## Architecture Direction

The project is split into three layers:

1. `core`
   Shared domain models, provider contracts, refresh orchestration, caching, redaction, and settings schema.
2. `providers`
   Provider-specific auth readers and usage fetchers behind narrow interfaces.
3. `desktop shell`
   Tauri desktop app with platform adapters for tray, window behavior, startup, key storage, and updater.

This lets us ship macOS first without locking the whole app to macOS-only implementation details.

## Security Principles

- Minimize credential access scope per provider.
- Default provider adapters to read-only behavior.
- Separate provider auth access from UI code.
- Keep logs redacted and avoid raw token persistence.
- Require explicit opt-in for telemetry, local API, and auto-update.
- Avoid loading arbitrary third-party provider scripts at runtime.

## Initial Deliverables

- [docs/architecture.md](docs/architecture.md)
- [docs/roadmap.md](docs/roadmap.md)
- workspace scaffold for:
  - `apps/desktop`
  - `packages/core`
  - `packages/platform`
  - `packages/providers`

## Development Plan

### Milestone 0

Define product scope, supported providers, security defaults, and shared data model.

### Milestone 1

Implement macOS MVP on the final architecture, not on a temporary fork-only layout.

### Milestone 2

Add Windows shell and credential adapters while keeping provider logic shared.

### Milestone 3

Harden packaging, update flow, diagnostics, and regression coverage.

## Workspace Layout

```text
apps/
  desktop/
packages/
  core/
  platform/
  providers/
docs/
```

## Getting Started

1. Install workspace dependencies with `npm install`.
2. Run `npm run typecheck`.
3. Run `npm run dev:desktop`.
4. Run `npm run build` for a production bundle.

## Mobile Widget Sync

The desktop app can publish a sanitized usage snapshot to a Cloudflare relay so an Android app/widget can display the latest provider usage.

- Detailed setup: [docs/widget-sync-setup.md](docs/widget-sync-setup.md)
- Implementation notes: [docs/android-widget.md](docs/android-widget.md)
- Public docs site: https://wilgon456.github.io/ai-usage-dashboard-public/

Security notes:

- Do not commit `apps/android/app/google-services.json` to a public repository.
- Do not commit Firebase service-account JSON files or Cloudflare Worker secrets.
- Treat the generated `/v1/snapshots/<pairId>?token=<syncToken>` URL as a secret.

## Agent-Friendly Setup

On a fresh machine, start here:

1. Run `npm run setup`.
2. If setup stops, inspect with `npm run doctor` or `npm run doctor:json`.
3. Run `npm run smoke`.
4. Start the app with `npm run dev:tauri`.

Additional details are in [docs/agent-setup.md](docs/agent-setup.md).

## Repository Status

This repository now contains:

- the initial architecture and roadmap
- a typed workspace for `core`, `platform`, and `providers`
- a React/Vite desktop shell in `apps/desktop`
- local-storage-backed development runtime for platform settings and demo credentials
- a live Codex usage bridge that reads local Codex auth and token-count session logs
- an initial Tauri v2 shell in `apps/desktop/src-tauri`

## Current OpenAI Integration

The current live integration is for `Codex`.

- Auth source: `~/.codex/auth.json`
- Remote usage source: ChatGPT/Codex backend usage endpoint
- Token usage source: local Codex session JSONL logs in `~/.codex/sessions`

This path is practical, but not based on a public stable OpenAI API contract. It should be treated as an implementation detail that may need updates if upstream behavior changes.

## Native Shell Status

`apps/desktop/src-tauri` is now scaffolded for a Tauri v2 desktop runtime.

- Frontend: Vite + React
- Native bridge: `get_codex_usage`
- Dev entry: `npm run dev:tauri`

The Tauri command path reads the same local Codex auth and session files as the dev-server bridge, so the app can move away from browser-only `/api` middleware.
