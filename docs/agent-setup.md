# Agent Setup

This repository is prepared so an agent can bring it to a runnable state with a small number of commands.

## Goal

Make first-time setup predictable on a clean machine:

1. inspect prerequisites
2. install missing development dependencies
3. install workspace packages
4. run verification
5. launch the Tauri app

## Entry Points

Use these commands from the repository root:

```bash
npm run setup
```

If setup stops or an agent wants structured diagnostics:

```bash
npm run doctor
```

```bash
npm run doctor:json
```

macOS:

```bash
npm run setup:macos
```

Windows PowerShell:

```powershell
npm run setup:windows
```

## What The Setup Scripts Handle

macOS:

- checks Xcode Command Line Tools
- installs Homebrew if missing
- installs Node.js if missing
- installs Rust via rustup if missing
- installs `gh`
- installs Claude Code via Homebrew cask
- installs Codex via npm
- runs `npm install`
- runs `npm run doctor`
- runs `npm run smoke`

Windows:

- self-elevates to administrator
- installs Visual Studio Build Tools with the Desktop development with C++ workload
- installs WebView2 Runtime
- installs Node.js LTS with `winget` if missing
- installs Rustup with `winget` and selects `stable-msvc`
- installs `gh`
- installs Claude Code
- installs Codex via npm
- runs `npm install`
- runs `npm run doctor`
- runs `npm run smoke`

## Remaining Interactive Steps

- provider login approval
- first-run OS dialogs such as Xcode Command Line Tools prompts on macOS
- rare machine-specific repair cases if WebView2 or Visual Studio installers fail upstream

## After Repo Setup

Run:

```bash
npm run dev:tauri
```

Then use the in-app provider bootstrap flow to install or sign in to:

- Claude
- Codex
- Copilot

OpenRouter remains API-key based.
