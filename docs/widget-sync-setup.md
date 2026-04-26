# AI Usage Dashboard 모바일 위젯 싱크 설치/세팅 가이드

이 문서는 데스크탑 AI Usage Dashboard에서 수집한 사용량 snapshot을 Android 위젯으로 동기화하는 방법을 설명합니다.

> 핵심 구조: 데스크탑 앱이 provider 사용량을 새로고침 → Cloudflare Relay에 안전한 snapshot 업로드 → 모바일 앱/위젯이 sync URL로 snapshot을 가져옴 → Android는 FCM wake signal + WorkManager polling으로 자동 갱신합니다.

## 1. 자동 갱신이 되는 조건

- Cloudflare Worker relay는 배포되어 있으면 계속 켜져 있습니다.
- 하지만 **새 사용량 snapshot을 만드는 주체는 데스크탑 앱**입니다.
- 따라서 최신 값으로 계속 갱신되려면 PC에서 AI Usage Dashboard 데스크탑 앱이 실행 중이어야 하고, 앱이 provider refresh를 수행해야 합니다.
- PC 앱이 꺼져 있으면 모바일은 마지막으로 relay에 올라간 snapshot 또는 Android에 저장된 마지막 snapshot만 보여줍니다.
- Android는 다음 경우에 sync를 시도합니다.
  - 앱을 열 때
  - `Sync Widget` 버튼을 누를 때
  - 홈 화면 위젯 update/package replace 이벤트가 올 때
  - WorkManager periodic sync가 돌 때
  - FCM wake signal을 받을 때

정리하면, **PC 서버/데스크탑 앱이 켜져 있고 refresh가 계속 돌면 모바일도 계속 따라옵니다.** 단, FCM/Android 배터리 정책 때문에 완전한 실시간 보장은 아니며, polling fallback으로 eventual sync를 보장합니다.

## 2. 필요한 것

### 데스크탑/개발 환경

- macOS 또는 Windows 데스크탑
- Node.js / npm
- Rust / Cargo
- Android Studio 또는 Android SDK/Gradle
- Cloudflare 계정 및 Wrangler 로그인
- Firebase project

### Android 기기

- Android 앱 APK 설치 가능 상태
- 홈 화면 위젯 추가 가능 런처
- 네트워크 연결

## 3. 저장소 준비

```bash
git clone <repository-url>
cd ai-usage-dashboard
npm install
npm run typecheck
cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml
```

개발 모드 실행은 실제 사용자 HOME을 명시하는 것이 안전합니다. provider credential은 보통 실제 사용자 홈 디렉터리에 있기 때문입니다.

```bash
HOME="$HOME" npm run dev:tauri
```

macOS에서 Hermes/자동화 환경 안에서 실행한다면 다음처럼 실제 계정 HOME을 명시하세요.

```bash
HOME=/Users/<you> npm run dev:tauri
```

## 4. Firebase Android 앱 설정

1. Firebase Console에서 project를 만듭니다.
2. Android app을 추가합니다.
3. package name은 앱의 namespace와 같아야 합니다.

```text
com.aiusagedashboard.widget
```

4. Firebase에서 `google-services.json`을 다운로드합니다.
5. 아래 위치에 넣습니다.

```text
apps/android/app/google-services.json
```

주의:

- `google-services.json`은 public repo에 커밋하지 마세요.
- 이 파일이 없으면 debug build는 가능하지만 FCM push token 등록은 동작하지 않습니다.
- 이 파일이 있으면 Gradle이 `com.google.gms.google-services` plugin을 조건부로 적용합니다.

## 5. Cloudflare Relay 설정

Cloudflare relay는 데스크탑이 업로드한 snapshot을 모바일이 가져갈 수 있도록 보관합니다.

### 5.1 Worker 배포

```bash
npm run deploy:relay:cf
```

배포 후 Worker base URL을 기록합니다.

```text
https://<your-worker>.<your-account>.workers.dev
```

### 5.2 FCM service account secrets 등록

Firebase Console에서 service account JSON을 발급한 뒤, 값 자체를 코드나 문서에 넣지 말고 Cloudflare Worker secret으로만 등록합니다.

필요한 secret:

```text
FCM_PROJECT_ID
FCM_CLIENT_EMAIL
FCM_PRIVATE_KEY
```

예시:

```bash
npx wrangler secret put FCM_PROJECT_ID
npx wrangler secret put FCM_CLIENT_EMAIL
npx wrangler secret put FCM_PRIVATE_KEY
```

주의:

- private key JSON 내용을 터미널 로그/문서/README/커밋에 남기지 마세요.
- public repo에는 service account JSON을 절대 넣지 마세요.
- Cloudflare secret 등록 후 Worker를 다시 배포하거나 재확인하세요.

## 6. Android APK 빌드/설치

```bash
npm run build:android
```

APK 위치:

```text
apps/android/app/build/outputs/apk/debug/app-debug.apk
```

ADB 설치:

```bash
adb install -r apps/android/app/build/outputs/apk/debug/app-debug.apk
```

필요하면 downgrade 허용:

```bash
adb install -r -d apps/android/app/build/outputs/apk/debug/app-debug.apk
```

## 7. 데스크탑 앱에서 sync URL 만들기

1. 데스크탑 앱을 엽니다.
2. provider 연결 상태를 확인합니다.
3. `Settings`로 이동합니다.
4. `System` 섹션으로 이동합니다.
5. `Android widget sync`를 켭니다.
6. `Relay URL`에 Worker base URL을 입력합니다.

```text
https://<your-worker>.<your-account>.workers.dev
```

7. 앱이 표시하는 전체 sync URL을 복사합니다.

형태는 다음과 같습니다.

```text
https://<your-worker>.<your-account>.workers.dev/v1/snapshots/<pairId>?token=<syncToken>
```

중요:

- 모바일 앱에는 반드시 `token=`까지 포함된 전체 URL을 붙여넣어야 합니다.
- 이 URL은 접근 토큰을 포함하므로 채팅/이슈/스크린샷에 그대로 공개하지 마세요.

## 8. Android 앱과 위젯 연결

1. Android 앱을 실행합니다.
2. 데스크탑 앱에서 복사한 전체 sync URL을 입력합니다.
3. `Sync Widget`을 누릅니다.
4. 성공 메시지가 나오면 홈 화면에 위젯을 추가합니다.
5. 이후부터는 다음 경로로 자동 갱신됩니다.
   - Android 앱 open 시 자동 sync
   - WorkManager periodic sync
   - 위젯 update 이벤트
   - Cloudflare relay의 FCM wake signal

## 9. 동작 방식 상세

```text
Desktop app
  refreshes provider usage
  builds sanitized snapshot
  PUT /v1/snapshots/:pairId Authorization: Bearer <syncToken>

Cloudflare relay
  stores latest sanitized snapshot
  sends wake-only FCM message when snapshot etag changes

Android app/widget
  receives wake signal
  enqueues WidgetSyncWorker one-shot
  GET /v1/snapshots/:pairId?token=<syncToken>
  stores snapshot in SharedPreferences
  refreshes home-screen widget
```

FCM payload에는 provider 사용량, sync URL, sync token이 들어가지 않습니다. push는 단지 “새 snapshot이 있으니 fetch해라”라는 wake signal입니다.

## 10. 문제 해결

### 위젯이 갱신되지 않음

- Android 앱에서 `Sync Widget`을 한 번 수동으로 눌러 저장된 URL을 확인하세요.
- sync URL에 `token=` 값이 포함되어 있는지 확인하세요.
- Worker URL이 `/v1/snapshots/...` 형태인지 확인하세요.
- Android 기기의 배터리 최적화가 WorkManager/FCM을 지연시킬 수 있습니다.

### 데스크탑 앱에서 provider가 연결 필요로 보임

- 데스크탑 앱이 실제 사용자 HOME으로 실행 중인지 확인하세요.
- Codex/Kimi/Copilot/OpenRouter credential은 보통 사용자 홈, keychain, env, CLI auth에 의존합니다.
- 자동화 환경에서 실행했다면 다음처럼 실행하세요.

```bash
HOME=/Users/<you> npm run dev:tauri
```

### FCM push가 안 옴

- `google-services.json`이 Android 앱 경로에 있는지 확인하세요.
- Firebase package name이 `com.aiusagedashboard.widget`인지 확인하세요.
- Cloudflare Worker에 `FCM_PROJECT_ID`, `FCM_CLIENT_EMAIL`, `FCM_PRIVATE_KEY` secret이 등록되어 있는지 확인하세요.
- push가 실패해도 polling으로 eventual sync가 됩니다.

### Cloudflare Worker는 응답하는데 snapshot이 없음

처음에는 데스크탑 앱이 한 번 provider refresh와 snapshot upload를 해야 합니다. 데스크탑 앱에서 Android widget sync를 켠 뒤 provider refresh를 실행하세요.

## 11. 보안 체크리스트

- [ ] `google-services.json`을 public repo에 커밋하지 않았습니다.
- [ ] Firebase service account JSON을 repo에 커밋하지 않았습니다.
- [ ] Cloudflare secrets는 `wrangler secret put`으로만 등록했습니다.
- [ ] sync URL 원문을 공개 이슈/문서/스크린샷에 노출하지 않았습니다.
- [ ] public 문서에는 `token=<syncToken>` 같은 placeholder만 사용했습니다.

## 12. 현재 한계

- Android 앱은 provider를 직접 조회하는 standalone client가 아니라, 데스크탑/relay snapshot viewer입니다.
- PC가 꺼져 있으면 새 provider 사용량은 생성되지 않습니다.
- FCM은 near-instant best-effort이며, Android/OEM 정책에 따라 지연될 수 있습니다.
- iPhone/iOS 위젯은 같은 snapshot contract로 확장 가능하지만 별도 iOS 앱/APNs 구현이 필요합니다.
