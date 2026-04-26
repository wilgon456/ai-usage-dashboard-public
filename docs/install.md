# Install

## macOS

1. Download `AI Usage Dashboard_0.1.0_aarch64.dmg` from the latest release.
2. Open the DMG and drag the app to Applications.
3. First launch: right-click the app in Applications, choose **Open**, then confirm.
   macOS Gatekeeper blocks unsigned apps by default during the MVP phase.
4. The app lives in the menu bar. Click the tray icon to show or hide the panel.

## Windows

Windows MSI artifacts are built via GitHub Actions on the `windows-latest` runner. See [.github/workflows/release.yml](../.github/workflows/release.yml).

1. Download `AI Usage Dashboard_0.1.0_x64_en-US.msi` from the latest release.
2. Double-click to install. SmartScreen may warn; click **More info** and then **Run anyway**.
3. Launch from the Start Menu. The app appears in the system tray.

## Authentication

- `Codex`: requires a logged-in `~/.codex/auth.json` from `codex login`.
- `Claude`: requires the `claude` CLI to be logged in (keychain entry or `~/.claude/.credentials.json`).
- `Copilot`: requires `gh auth login` with Copilot access.
- `OpenRouter`: requires `OPENROUTER_API_KEY` or a key saved through the in-app Connect modal.

On macOS the first refresh after login prompts for keychain access once; the app caches successful reads in-process so repeated refreshes no longer re-prompt.

The in-app UI is currently localized to Korean. A locale bundle extraction is tracked under "Remaining tasks" in [`status-ko.md`](./status-ko.md).

## Uninstall

- macOS: drag the app from Applications to Trash. Remove any related keychain entries if desired.
- Windows: go to Settings, open Apps, select AI Usage Dashboard, then uninstall. Credentials remain in Credential Manager.
