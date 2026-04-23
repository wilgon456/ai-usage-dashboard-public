# AI Usage Dashboard

Cross-platform desktop dashboard for AI coding assistant usage.

## What It Does

`AI Usage Dashboard` is a Tauri desktop app that shows usage and account state for:

- Codex
- Claude
- GitHub Copilot
- OpenRouter

The app is built as a small tray / menu bar surface rather than a browser-first dashboard.

## Current Scope

- macOS tray / menu bar workflow
- Windows system tray packaging target
- Tauri v2 desktop shell
- React + Vite frontend
- Shared provider/core packages

## Project Layout

```text
apps/
  desktop/
packages/
  core/
  platform/
  providers/
docs/
```

## Development

1. Install dependencies:

```bash
npm install
```

2. Run type checks:

```bash
npm run typecheck
```

3. Run the desktop app in dev mode:

```bash
npm run dev:tauri
```

4. Build the desktop app:

```bash
npm run build
```

## Packaging

- macOS DMG:

```bash
npm run build:desktop:mac
```

- Windows MSI:

```bash
npm run build:desktop:win
```

Windows release artifacts are also produced by GitHub Actions.

## Notes

- Provider integrations rely on local auth/session state exposed by each vendor CLI or API key flow.
- Some usage sources are based on vendor-private or unstable response contracts and may require maintenance if upstream behavior changes.

## Docs

- [Architecture](docs/architecture.md)
- [Install](docs/install.md)
- [Roadmap](docs/roadmap.md)
