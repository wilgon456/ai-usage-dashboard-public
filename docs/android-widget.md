# Android Widget

The Android work starts as a separate native app in `apps/android`.

This is intentional. The current desktop app depends on local desktop provider credentials and CLI/session files, while Android needs a different auth and sync model. The widget reads a compact JSON snapshot from Android `SharedPreferences`; later work can replace the demo writer with a companion sync endpoint, cloud sync, or mobile-native provider adapters without changing the widget rendering contract.

## Current MVP

- Native Android application module at `apps/android/app`
- Home screen widget backed by `UsageWidgetProvider`
- Local snapshot boundary in `UsageSnapshotStore`
- Manual cloud relay sync URL import for `https://relay.example.com/v1/snapshots/...?...`
- Automatic Android sync from the saved sync URL when:
  - the launch activity opens
  - the home-screen widget receives an update event
  - WorkManager runs the periodic sync job
- Minimal launch activity with:
  - `Sync Widget`
  - `Seed Demo Snapshot`
  - `Refresh Widgets`

The desktop app uploads widget sync only when Settings -> System -> Android widget sync is enabled. The relay endpoint is token-protected and returns a safe JSON snapshot without credentials.

The demo snapshot is generated from `packages/core/src/domain/provider-registry.json`, the same registry that defines the desktop default provider order.

The generated snapshot format is intentionally close to the dashboard card model:

```json
{
  "fetchedAt": "2026-04-24T00:00:00Z",
  "providers": [
    {
      "id": "codex",
      "name": "Codex",
      "percentUsed": 42,
      "summary": "128K tokens today",
      "accentColor": "#111827"
    },
    {
      "id": "copilot",
      "name": "Copilot",
      "percentUsed": 68,
      "summary": "Monthly included usage",
      "accentColor": "#0f766e"
    },
    {
      "id": "openrouter",
      "name": "OpenRouter",
      "percentUsed": 23,
      "summary": "$4.28 this month",
      "accentColor": "#2563eb"
    },
    {
      "id": "kimi",
      "name": "Kimi",
      "percentUsed": 36,
      "summary": "Connected via CLI",
      "accentColor": "#7c3aed"
    }
  ]
}
```

## Build

From the repository root:

```sh
npm run build:android
```

This expects a local Android SDK and Gradle installation. If this project later adopts a checked-in Gradle wrapper, update the script to call `apps/android/gradlew`.

## Cloud Relay Sync

### Cloudflare Workers

The recommended free path is Cloudflare Workers + KV.

1. Create a KV namespace:

```sh
npx wrangler kv namespace create WIDGET_SNAPSHOTS --config apps/relay-cloudflare/wrangler.toml
```

2. Put the returned namespace id into `apps/relay-cloudflare/wrangler.toml`.
3. Deploy:

```sh
npm run deploy:relay:cf
```

4. Open the desktop app.
5. Go to Settings -> System.
6. Enable Android widget sync.
7. Enter the deployed Worker base URL, for example `https://ai-usage-dashboard-relay.<account>.workers.dev`.
8. Copy the displayed `/v1/snapshots/...` URL into the Android app.
9. Tap `Sync Widget` once, or reopen the Android app. After a sync URL is saved, Android schedules periodic WorkManager sync and widget update events also enqueue a fresh fetch.

Notes:
- Android still keeps WorkManager polling as the fallback, but the Cloudflare relay can now send an FCM wake signal right after a changed PC snapshot upload.
- Push is intentionally a wake signal only. The push payload contains `type`, `pairId`, `updatedAt`, `snapshotEtag`, and `schemaVersion`; it never contains provider usage data or the sync token.
- Android and OEM battery policy can still delay delivery, so this is near-instant best-effort rather than a hard real-time guarantee.
- The Android app is currently a desktop/relay snapshot viewer. Full standalone provider support requires separate mobile credential/auth implementations per provider.

## Relay + Push

The push-enhanced path keeps the existing snapshot contract:

```text
Desktop app
  PUT /v1/snapshots/:pairId  Authorization: Bearer <syncToken>
Cloudflare relay
  stores sanitized snapshot
  sends FCM data message when the snapshot etag changes
Android app/widget
  receives snapshot.updated
  enqueues WidgetSyncWorker one-shot
  GET /v1/snapshots/:pairId?token=<syncToken>
```

### Push registration API

After the Android app has a saved relay sync URL, it attempts to register its FCM token:

```http
POST /v1/push/:pairId/register
Authorization: Bearer <syncToken>
Content-Type: application/json

{
  "platform": "android",
  "provider": "fcm",
  "pushToken": "...",
  "appVersion": "0.1.0",
  "deviceId": "locally-generated-random-id"
}
```

To remove a device token:

```http
POST /v1/push/:pairId/unregister
Authorization: Bearer <syncToken>
Content-Type: application/json

{
  "platform": "android",
  "provider": "fcm",
  "pushToken": "..."
}
```

The relay stores devices under per-token KV keys (`push:${pairId}:...`) so registration is idempotent and avoids array overwrite races. The snapshot is stored under `snapshot:${pairId}`. Legacy direct `pairId` snapshot reads remain supported for old KV entries.

### FCM configuration

For local tests, the Worker supports `FCM_ACCESS_TOKEN` and `FCM_SEND_URL`. For real Cloudflare deployment, prefer service-account based secrets:

```sh
npx wrangler secret put FCM_PROJECT_ID --config apps/relay-cloudflare/wrangler.toml
npx wrangler secret put FCM_CLIENT_EMAIL --config apps/relay-cloudflare/wrangler.toml
npx wrangler secret put FCM_PRIVATE_KEY --config apps/relay-cloudflare/wrangler.toml
```

`FCM_PRIVATE_KEY` may contain escaped newlines (`\\n`). If the FCM secrets are absent, snapshot upload still succeeds and push delivery is treated as best-effort failure; WorkManager polling continues to work.

Android requires Firebase Messaging runtime configuration for actual token issuance. Put the Firebase client config at `apps/android/app/google-services.json`; the Gradle build applies `com.google.gms.google-services` only when that file exists, so local/debug builds without Firebase config still compile and keep polling active.

### iOS/APNs contract

iOS can reuse the same API contract later:

```json
{
  "platform": "ios",
  "provider": "apns",
  "pushToken": "<apns device token>",
  "appVersion": "0.1.0",
  "deviceId": "locally-generated-random-id"
}
```

APNs delivery is not implemented yet. The relay data model already accepts `ios`/`apns`, but send-side support should be added with APNs provider-token credentials and the same wake-only payload rule.

### Node Relay

For non-Cloudflare hosts, the Node relay remains available:

```sh
npm run dev:relay
```

When a provider is disabled on desktop, it is omitted from the next uploaded widget snapshot. Android reflects the change after the next sync.

The legacy LAN URL is still shown as a development fallback, but the cloud relay URL is the useful mobile path.

## Next Decisions

1. Choose the data source:
   - backend sync service
   - Android-native provider adapters
2. Add automatic background sync on Android.
3. Add a widget configuration screen for provider ordering and display mode.
4. Replace manual relay URL entry with QR/pairing-code setup.
