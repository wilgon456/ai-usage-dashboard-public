# Install

## macOS

1. Download the latest DMG artifact from GitHub Releases.
2. Open the DMG and drag the app into `Applications`.
3. On first launch, macOS may require the usual Gatekeeper confirmation for unsigned or newly signed builds.
4. The app runs from the menu bar.

## Windows

1. Download the latest MSI artifact from GitHub Releases.
2. Run the installer.
3. If SmartScreen appears, continue through the standard confirmation flow.
4. The app runs from the system tray.

## Provider Authentication

- Codex: requires a logged-in local Codex CLI session.
- Claude: requires a logged-in Claude Code session or supported auth token path.
- Copilot: requires `gh auth login` with Copilot access.
- OpenRouter: requires an API key saved in-app or provided through `OPENROUTER_API_KEY`.

## Uninstall

- macOS: remove the app from `Applications`.
- Windows: uninstall from Apps / Installed Apps.
