# Bootstrap Flow

This app now uses a bootstrap-first connection flow for CLI-based providers.

## Goal

Reduce setup failures on fresh machines by turning provider setup into a guided sequence:

1. Diagnose the local environment.
2. Identify missing prerequisites.
3. Launch one terminal flow that installs what is missing.
4. Continue into the provider login step.
5. Return to the dashboard and refresh usage.

## Current Scope

CLI bootstrap currently applies to:

- Claude
- Codex
- Copilot

OpenRouter remains API-key based and does not use the bootstrap flow.

## Runtime Contract

The frontend asks Tauri for `inspect_provider_bootstrap` before showing the primary action.

The backend returns:

- step list
- per-step status: `ready`, `action_required`, `unavailable`
- whether automatic execution is possible
- whether a launchable bootstrap command exists

The same planner is reused by `run_connect_command`, so the UI summary and the executed command stay aligned.

## Platform Strategy

macOS:

- Claude: native installer or Homebrew cask
- Copilot: Homebrew + `gh auth login --web`
- Codex: Homebrew/Node.js + `npm install -g @openai/codex`

Windows:

- Claude: official PowerShell installer
- Copilot: `winget` + `gh auth login --web`
- Codex: `winget` Node.js + `npm install -g @openai/codex`

## Known Limits

- Some installers still require user approval, admin rights, or a shell restart.
- Windows path propagation after installing Node.js or CLI tools can still vary by machine.
- This flow automates installation orchestration; it does not guarantee silent installation.
