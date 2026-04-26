# Android 위젯 구현 노트

이 문서는 AI Usage Dashboard의 Android 앱/홈 화면 위젯이 어떻게 동작하는지 설명합니다. 일반 사용자가 설치만 하려면 [모바일 위젯 싱크 설치/세팅 가이드](widget-sync-setup.md)를 먼저 보세요.

## 왜 별도 Android 앱인가?

현재 데스크탑 앱은 로컬 PC의 provider credential, CLI session file, keychain/env 등을 이용해 사용량을 조회합니다. Android에서 같은 credential을 그대로 읽을 수 없기 때문에 Android 앱은 provider를 직접 조회하지 않고, 데스크탑 앱이 만든 **sanitized snapshot**을 받아 표시합니다.

즉 현재 Android 앱은 다음 역할입니다.

```text
데스크탑 provider 조회 결과를 모바일에서 보는 snapshot viewer + 홈 화면 위젯
```

## 현재 MVP 구성

- Android module: `apps/android/app`
- 홈 화면 위젯: `UsageWidgetProvider`
- snapshot 저장소: `UsageSnapshotStore`
- HTTP fetch/검증/저장 helper: `UsageSnapshotSync`
- background sync worker: `WidgetSyncWorker`
- FCM token 등록 helper: `WidgetPushRegistrar`
- FCM 수신 service: `UsageFirebaseMessagingService`
- 최소 Android activity:
  - sync URL 입력
  - `Sync Widget`
  - `Seed Demo Snapshot`
  - `Refresh Widgets`

## 데이터 흐름

```text
Desktop app
  provider usage refresh
  sanitized widget snapshot 생성
  PUT /v1/snapshots/:pairId Authorization: Bearer <syncToken>

Cloudflare Relay
  최신 snapshot 저장
  snapshot etag가 바뀌면 FCM wake signal 전송

Android app/widget
  FCM wake signal 수신
  WidgetSyncWorker one-shot enqueue
  GET /v1/snapshots/:pairId?token=<syncToken>
  SharedPreferences에 snapshot 저장
  홈 화면 위젯 갱신
```

## Snapshot 형식

위젯은 provider credential이나 raw token을 저장하지 않습니다. 표시용 필드만 저장합니다.

```json
{
  "schemaVersion": 1,
  "fetchedAt": "2026-04-24T00:00:00Z",
  "providers": [
    {
      "id": "codex",
      "name": "Codex",
      "percentUsed": 42,
      "usageLabel": "128K tokens today",
      "summary": "128K tokens today",
      "accentColor": "#111827",
      "state": "normal"
    }
  ]
}
```

## Android 쪽 자동 sync trigger

Android는 다음 상황에서 sync를 시도합니다.

- 앱 실행 시 저장된 sync URL이 있으면 자동 sync
- 사용자가 `Sync Widget` 버튼을 누를 때
- 홈 화면 위젯 update 이벤트가 올 때
- 앱 package replace 이벤트가 올 때
- WorkManager periodic sync가 돌 때
- FCM wake signal을 받을 때

FCM이 지연되거나 실패해도 WorkManager polling이 fallback으로 동작합니다.

## Cloudflare Relay + Push

### Snapshot API

데스크탑 앱은 최신 snapshot을 relay에 업로드합니다.

```http
PUT /v1/snapshots/:pairId
Authorization: Bearer <syncToken>
Content-Type: application/json
```

Android 앱은 저장된 URL로 snapshot을 가져옵니다.

```http
GET /v1/snapshots/:pairId?token=<syncToken>
```

### Push 등록 API

Android 앱은 sync URL이 저장된 뒤 FCM token을 relay에 등록합니다.

```http
POST /v1/push/:pairId/register
Authorization: Bearer <syncToken>
Content-Type: application/json

{
  "platform": "android",
  "provider": "fcm",
  "pushToken": "<fcm device token>",
  "appVersion": "0.1.0",
  "deviceId": "locally-generated-random-id"
}
```

등록 해제 API도 있습니다.

```http
POST /v1/push/:pairId/unregister
Authorization: Bearer <syncToken>
Content-Type: application/json

{
  "platform": "android",
  "provider": "fcm",
  "pushToken": "<fcm device token>"
}
```

Relay는 device token을 pair별 KV key로 저장합니다. 중복 등록은 idempotent하게 처리됩니다.

## FCM payload 원칙

FCM payload는 wake signal만 담습니다.

포함 가능:

- `type=snapshot.updated`
- `pairId`
- `updatedAt`
- `snapshotEtag`
- `schemaVersion`

포함 금지:

- provider 사용량 데이터
- sync URL
- sync token
- provider token/API key

Android는 push payload의 데이터로 화면을 직접 갱신하지 않고, push를 받으면 `WidgetSyncWorker`를 깨워 relay에서 snapshot을 다시 가져옵니다.

## Firebase 설정

Android에서 실제 FCM token을 받으려면 Firebase client config가 필요합니다.

```text
apps/android/app/google-services.json
```

이 파일이 있으면 Gradle build가 `com.google.gms.google-services` plugin을 조건부로 적용합니다. 파일이 없어도 debug build는 가능하며, 이 경우 push 등록은 동작하지 않고 polling fallback만 사용합니다.

## Worker secret

Cloudflare Worker에서 FCM을 보내려면 service-account 기반 secret이 필요합니다.

```bash
npx wrangler secret put FCM_PROJECT_ID --config apps/relay-cloudflare/wrangler.toml
npx wrangler secret put FCM_CLIENT_EMAIL --config apps/relay-cloudflare/wrangler.toml
npx wrangler secret put FCM_PRIVATE_KEY --config apps/relay-cloudflare/wrangler.toml
```

`FCM_PRIVATE_KEY`는 escaped newline(`\\n`)을 포함할 수 있습니다. Worker 코드는 이를 실제 newline으로 복원해 JWT를 만듭니다.

## iOS/APNs 확장 가능성

Relay data model은 나중에 iOS/APNs 등록을 받을 수 있도록 `ios` / `apns` 형태를 받아들일 수 있습니다.

```json
{
  "platform": "ios",
  "provider": "apns",
  "pushToken": "<apns device token>",
  "appVersion": "0.1.0",
  "deviceId": "locally-generated-random-id"
}
```

다만 APNs 전송 자체는 아직 구현되어 있지 않습니다. iPhone 위젯까지 지원하려면 별도 iOS 앱, WidgetKit, APNs provider token 구현이 필요합니다.

## Node Relay

Cloudflare가 아닌 환경에서 테스트하고 싶다면 Node relay를 사용할 수 있습니다.

```bash
npm run dev:relay
```

실제 모바일 사용에는 Cloudflare relay가 더 적합합니다. 로컬 LAN URL은 개발 fallback 성격입니다.

## 보안 체크포인트

- sync URL은 token을 포함하므로 secret으로 취급합니다.
- Android error message는 sync URL과 `token=` 값을 redact합니다.
- Widget snapshot에는 provider credential을 넣지 않습니다.
- Push payload에는 provider 사용량과 token을 넣지 않습니다.
- Public repo에는 `google-services.json`과 Firebase service account JSON을 넣지 않습니다.

## 다음 개선 후보

- QR code 또는 pairing code 기반 연결
- Android 위젯 설정 화면에서 provider 순서/display mode 선택
- stale FCM token 정리 강화
- iOS WidgetKit/APNs 지원
- Android-native provider adapter 실험
