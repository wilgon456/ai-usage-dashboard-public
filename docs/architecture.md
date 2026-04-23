# Architecture

The project is split into three layers:

1. `packages/core`
   Shared domain types, snapshot models, settings, and refresh orchestration.

2. `packages/providers`
   Provider-specific adapters that fetch usage/account state and normalize it into the shared snapshot shape.

3. `apps/desktop`
   Tauri desktop shell plus the React frontend used for tray / menu bar UI.

## Design Goals

- Cross-platform desktop-first architecture
- Narrow provider auth boundaries
- Read-only provider integrations wherever possible
- Minimal telemetry by default
- Small tray-focused user experience

## Runtime Notes

- The frontend is React + Vite.
- The native shell is Tauri v2.
- Credentials are resolved through provider-specific native adapters.
- Some providers use local CLI state rather than public stable APIs.

## Platform Direction

- macOS: menu bar style workflow
- Windows: system tray workflow and MSI packaging

The codebase is organized so provider logic stays shared while platform-specific shell behavior can diverge where necessary.
