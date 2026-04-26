# AI 에이전트용 설치/세팅 프롬프트

아래 내용을 그대로 AI 코딩 에이전트에게 전달하면, AI Usage Dashboard를 clone부터 Android 위젯 sync까지 세팅하는 작업 지시로 사용할 수 있습니다.

---

## 에이전트에게 전달할 프롬프트

```text
목표: AI Usage Dashboard를 설치하고, 데스크탑 앱에서 Android 앱/위젯으로 사용량 snapshot이 sync되도록 세팅해줘.

중요 보안 규칙:
- Firebase service account JSON, google-services.json, Cloudflare secret, provider token/API key 값을 절대 출력하지 마.
- 비밀 값은 존재 여부, 파일 경로, 필드 존재 여부, 길이 정도만 확인해.
- `/v1/snapshots/<pairId>?token=<syncToken>` 형태의 전체 sync URL은 비밀번호처럼 취급해. 공개 로그/문서/이슈에 원문을 남기지 마.
- public repo에 `google-services.json`, service account JSON, `.env`, provider token을 커밋하지 마.

작업 순서:

1. 저장소 준비
   - `git clone <repository-url>`
   - `cd ai-usage-dashboard`
   - `npm install`

2. 기본 검증
   - `npm run typecheck`
   - `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`
   - `npm run build:android`
   - 실패하면 에러 메시지를 먼저 읽고 root cause를 찾은 뒤 수정해.

3. Cloudflare Relay 준비
   - `apps/relay-cloudflare/wrangler.toml` 확인
   - KV namespace가 필요하면 Wrangler로 생성하고 binding을 설정해.
   - Worker 배포는 `npm run deploy:relay:cf`로 수행해.
   - Worker base URL을 기록하되, secret은 출력하지 마.

4. Firebase Android 설정
   - Firebase Console에서 Android app package가 `com.aiusagedashboard.widget`인지 확인해.
   - 사용자가 받은 `google-services.json`을 `apps/android/app/google-services.json`에 배치해.
   - 이 파일은 public repo에 커밋하지 마.
   - `npm run build:android`로 `processDebugGoogleServices`가 정상 동작하는지 확인해.

5. FCM service account -> Cloudflare secret
   - Firebase service account JSON에서 필요한 값은 Cloudflare Worker secret으로만 넣어.
   - 필요한 secret 이름:
     - `FCM_PROJECT_ID`
     - `FCM_CLIENT_EMAIL`
     - `FCM_PRIVATE_KEY`
   - 값은 터미널 로그에 남기지 말고 `wrangler secret put`을 사용해.
   - 등록 후 Worker를 다시 배포하거나 동작을 확인해.

6. 데스크탑 앱 실행
   - 일반 실행: `npm run dev:tauri`
   - 자동화/Hermes/CI 비슷한 환경에서 provider credential을 못 찾으면 실제 HOME을 명시해:
     - macOS 예: `HOME=/Users/<user> npm run dev:tauri`
   - provider 카드가 연결 상태인지 확인해.

7. 데스크탑 앱에서 Android widget sync 켜기
   - Settings -> System -> Android widget sync 활성화
   - Relay URL에 Cloudflare Worker base URL 입력
   - 표시되는 전체 sync URL을 복사
   - URL 형태: `https://<worker>/v1/snapshots/<pairId>?token=<syncToken>`

8. Android 앱 설치 및 연결
   - APK 빌드: `npm run build:android`
   - APK 위치: `apps/android/app/build/outputs/apk/debug/app-debug.apk`
   - 설치: `adb install -r apps/android/app/build/outputs/apk/debug/app-debug.apk`
   - Android 앱 실행
   - 데스크탑 앱에서 복사한 전체 sync URL 입력
   - `Sync Widget` 버튼 클릭
   - 홈 화면에 AI Usage Dashboard 위젯 추가

9. 동작 확인
   - 데스크탑 앱에서 provider refresh
   - Cloudflare relay에 snapshot PUT 성공 여부 확인
   - Android 앱/위젯이 snapshot을 가져오는지 확인
   - FCM이 안 와도 WorkManager polling fallback이 있으므로 eventual sync가 되는지 확인

10. 마지막 보고
   - 변경한 파일
   - 실행한 검증 명령과 결과
   - 배포한 Worker URL
   - Android APK 위치
   - 남은 수동 작업
   - 비밀 값은 절대 포함하지 말 것
```

---

## 사람이 직접 확인해야 할 것

- Firebase project와 Android app package name이 맞는지
- Cloudflare 계정/Wrangler 로그인이 되어 있는지
- Android 기기에 APK 설치 권한이 있는지
- 데스크탑 앱에서 provider credential이 정상 연결되어 있는지

## 참고 문서

- [모바일 위젯 싱크 설치/세팅 가이드](widget-sync-setup.md)
- [Android 위젯 구현 노트](android-widget.md)
