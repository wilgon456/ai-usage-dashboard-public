# AI Usage Dashboard

AI Usage Dashboard는 여러 AI 서비스의 사용량을 한 화면에서 확인하는 데스크탑 앱입니다. 현재는 macOS 중심의 Tauri 앱과 Android 위젯 동기화 기능을 포함합니다.

> 이 저장소는 공개 배포용입니다. 개인 Firebase 설정, Cloudflare secret, provider token, 실제 sync URL은 포함하지 않습니다.

## 무엇을 할 수 있나요?

- Codex, Copilot, OpenRouter, Kimi 등 provider 사용량을 데스크탑에서 확인
- 데스크탑 앱이 만든 안전한 snapshot을 Cloudflare Relay로 업로드
- Android 앱/홈 화면 위젯에서 해당 snapshot을 표시
- FCM wake signal + Android WorkManager polling으로 모바일 위젯을 자동 갱신
- telemetry/local API는 기본 비활성화

## 현재 구조

```text
apps/
  desktop/             # Tauri + React 데스크탑 앱
  android/             # Android 앱 + 홈 화면 위젯
  relay-cloudflare/    # Cloudflare Worker relay
  relay/               # Node relay 개발용 서버
packages/
  core/                # provider/domain/settings 모델
  platform/            # platform abstraction
  providers/           # provider adapter 계약 및 구현
docs/                  # GitHub Pages 문서
```

## 빠른 시작

```bash
git clone <repository-url>
cd ai-usage-dashboard
npm install
npm run typecheck
cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml
npm run build:android
```

데스크탑 개발 실행:

```bash
npm run dev:tauri
```

macOS 자동화/에이전트 환경에서 실행할 때 provider credential을 못 찾으면 실제 사용자 HOME을 명시하세요.

```bash
HOME="$HOME" npm run dev:tauri
```

## 일반 사용자를 위한 설치/세팅 문서

GitHub Pages에서 단계별 문서를 볼 수 있습니다.

- 문서 홈: <https://wilgon456.github.io/ai-usage-dashboard-public/>
- 모바일 위젯 싱크 설치/세팅 가이드: <https://wilgon456.github.io/ai-usage-dashboard-public/widget-sync-setup.html>
- AI 에이전트에게 그대로 던질 설치 프롬프트: <https://wilgon456.github.io/ai-usage-dashboard-public/ai-agent-setup-prompt.html>
- Android 위젯 구현 노트: <https://wilgon456.github.io/ai-usage-dashboard-public/android-widget.html>

## 모바일 위젯 싱크 요약

1. Cloudflare Worker relay를 배포합니다.
2. Firebase Android 앱을 만들고 `google-services.json`을 로컬에 둡니다.
3. Firebase service account 값은 Cloudflare Worker secret으로만 등록합니다.
4. 데스크탑 앱에서 `Settings -> System -> Android widget sync`를 켭니다.
5. Worker base URL을 입력합니다.
6. 데스크탑 앱에 표시되는 전체 `/v1/snapshots/<pairId>?token=<syncToken>` URL을 Android 앱에 붙여넣습니다.
7. Android 앱에서 `Sync Widget`을 한 번 누르고 홈 화면 위젯을 추가합니다.

주의: generated sync URL은 접근 토큰을 포함합니다. public issue, README, screenshot에 그대로 올리지 마세요.

## AI 에이전트에게 맡기고 싶다면

새 컴퓨터에서 에이전트에게 작업을 맡길 때는 아래 문서를 통째로 전달하면 됩니다.

```text
https://wilgon456.github.io/ai-usage-dashboard-public/ai-agent-setup-prompt.html
```

해당 문서에는 clone, dependency 설치, 검증 명령, Firebase/Cloudflare 설정, Android build, 위젯 연결까지 순서대로 적혀 있습니다.

## 보안 원칙

- `apps/android/app/google-services.json`은 public repo에 커밋하지 않습니다.
- Firebase service account JSON은 repo에 커밋하지 않습니다.
- Cloudflare Worker secret은 `wrangler secret put`으로만 등록합니다.
- provider token/API key를 로그나 문서에 남기지 않습니다.
- FCM push payload에는 사용량 데이터나 sync token을 넣지 않고 wake signal만 보냅니다.

## 주요 검증 명령

```bash
npm run test --workspace @ai-usage-dashboard/relay-cloudflare
npm run typecheck
cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml
npm run build:android
```

## 현재 한계

- Android 앱은 provider를 직접 조회하는 standalone client가 아니라 데스크탑/relay snapshot viewer입니다.
- PC의 데스크탑 앱이 꺼져 있으면 새 snapshot은 만들어지지 않습니다.
- FCM은 near-instant best-effort이며 Android/OEM 배터리 정책에 따라 지연될 수 있습니다.
- iPhone/iOS 위젯은 같은 snapshot contract로 확장 가능하지만 별도 iOS 앱/APNs 구현이 필요합니다.

## 라이선스

아직 명시 라이선스가 없다면 공개 배포 전에 `LICENSE` 파일을 추가하는 것을 권장합니다.
