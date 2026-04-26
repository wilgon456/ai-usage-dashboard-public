# Session Handoff

Date: 2026-04-23

## Summary

This repository is now in a much better state for:

- desktop provider bootstrap
- agent-friendly repository bootstrap
- public-repo export preparation
- Claude/Copilot/OpenRouter usage stabilization

The next major product direction under discussion is an Android companion app, but that work has not started yet.

## Completed Work

### Desktop provider bootstrap

Implemented in-app bootstrap flows for CLI-based providers:

- Claude
- Codex
- Copilot

Key files:

- `apps/desktop/src-tauri/src/commands/connect.rs`
- `apps/desktop/src/components/ConnectionModal.tsx`
- `apps/desktop/src-tauri/src/lib.rs`

Behavior:

- inspect current machine state before launching
- show bootstrap steps in the connection modal
- launch install/login flows through terminal commands
- support macOS and Windows branches

### Agent-friendly repo bootstrap

Added repo-level setup and diagnostics so an agent can bring the workspace to a runnable state with minimal guessing.

Key files:

- `AGENTS.md`
- `scripts/setup-agent.mjs`
- `scripts/setup-macos.sh`
- `scripts/setup-windows.ps1`
- `scripts/doctor.mjs`
- `docs/agent-setup.md`
- `docs/bootstrap.md`

New scripts:

- `npm run setup`
- `npm run setup:macos`
- `npm run setup:windows`
- `npm run doctor`
- `npm run doctor:json`
- `npm run smoke`

### Claude/Copilot/OpenRouter stabilization

Previously completed and kept in place:

- Claude OAuth/keychain/env handling improved
- Claude API-mode fallback to local stats
- OpenRouter progress now based on `/api/v1/credits`
- Copilot now prefers `gh auth token`
- keychain warm-up removed at app startup
- compact provider card no longer renders status-only states as empty

Key files:

- `apps/desktop/src-tauri/src/commands/claude.rs`
- `apps/desktop/src-tauri/src/commands/openrouter.rs`
- `apps/desktop/src-tauri/src/credentials/claude.rs`
- `apps/desktop/src-tauri/src/credentials/copilot.rs`
- `apps/desktop/src-tauri/src/credentials/openrouter.rs`
- `apps/desktop/src/components/ProviderCard.tsx`

### Public repo work

Already created and pushed:

- public repo: `wilgon456/ai-usage-dashboard-public`
- public export directory exists at `public-release/`
- README in public repo translated to Korean

## Refactoring Done

Refactored `connect.rs` to reduce duplicated bootstrap logic.

Added:

- `PlannedBootstrapBuilder`
- shared auth step helper

Purpose:

- lower maintenance cost
- reduce platform-branch duplication
- make future provider bootstrap changes safer

## Validation Status

Most recent successful checks:

- `npm run doctor`
- `npm run doctor:json`
- `npm run smoke`
- `npm run typecheck`
- `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`
- `cargo clippy --manifest-path apps/desktop/src-tauri/Cargo.toml --all-targets -- -D warnings`
- `npm run build`

Current macOS doctor result was clean.

## Known Risks

### Operational risk: Claude token exposure in dev process list

Observed during session:

- the current `dev:tauri` process command line exposed a Claude OAuth token via environment variable

This is not a compile/runtime crash issue, but it is a real security risk.

Recommended follow-up:

1. rotate the exposed token
2. stop passing long-lived secrets in process-visible command lines
3. move to keychain/file-backed loading or a safer launcher flow

### Windows setup still needs real-host validation

The Windows bootstrap path is much stronger now, but it has not been fully validated on an actual Windows machine in this session.

High-value validation targets:

- Visual Studio Build Tools install path
- WebView2 install path
- PATH refresh after Node/Rust/CLI installs
- `npm run setup`
- `npm run dev:tauri`

### Android app is not started yet

The repo is still desktop-first.

Android direction is feasible, but it should be treated as:

- shared UI and domain reuse where possible
- mobile-specific runtime and auth model
- likely a companion/sync architecture rather than direct reuse of desktop CLI assumptions

## Suggested Next Steps

### If continuing desktop hardening

1. remove secret exposure from dev launch flow
2. validate full Windows bootstrap on a real Windows host
3. fix anything found during Windows real-machine testing

### If starting Android work

1. design Android companion architecture
2. separate desktop-only runtime assumptions from reusable app logic
3. decide between:
   - Tauri mobile extension of current app
   - separate `apps/mobile` app using shared packages

## Recommended Restart Prompt

If a future agent needs a short starting point, use:

`Read docs/session-handoff-2026-04-23.md first, then run npm run doctor and continue from the highest-risk unfinished item.`
