# AI Usage Dashboard

AI 코딩 도구의 사용량과 연결 상태를 한곳에서 확인하는 크로스플랫폼 데스크톱 대시보드입니다. Tauri v2, React, Vite, TypeScript, Rust로 구성되어 있으며 macOS 메뉴바와 Windows 시스템 트레이에서 작게 띄워 쓰는 흐름을 목표로 합니다.

> 현재 공개 릴리스는 **Codex, Claude, GitHub Copilot, OpenRouter**를 대상으로 합니다. 각 서비스의 공식/비공식 로컬 인증 상태와 사용량 응답을 읽어 화면에 표시합니다.

## 주요 기능

- **홈 대시보드**
  - 연결된 AI 목록과 각 AI의 현재 상태를 카드로 표시합니다.
  - `사용 / 잔여` 표시 옵션을 홈 카드의 퍼센트 숫자와 진행 바에 동일하게 적용합니다.
  - 홈 상단은 간결하게 `홈` 제목만 보여줍니다.

- **AI별 상세 화면**
  - 각 AI의 상세 사용량을 별도 화면에서 확인합니다.
  - 진행률, 잔여량, 리셋 시간, 캐시 상태, 인증 문제를 카드 형태로 표시합니다.

- **트레이 / 메뉴바 앱 경험**
  - macOS 메뉴바와 Windows 시스템 트레이 중심의 작은 데스크톱 앱입니다.
  - 트레이 아이콘/라벨은 사용량 기준으로 갱신됩니다.

- **사용량 알림**
  - 설정한 임계값을 넘으면 데스크톱 알림을 보낼 수 있습니다.
  - 기본 임계값은 80%, 95%입니다.

- **연결 가이드**
  - 각 provider의 CLI 로그인 또는 API 키 설정 흐름을 앱 안에서 안내합니다.
  - OpenRouter API 키는 앱 안에서 저장/교체/삭제할 수 있습니다.

## 이번 공개 업데이트에 포함된 개선

- 홈 카드에서 `사용 / 잔여` 옵션 변경 시 퍼센트 숫자뿐 아니라 진행 바 너비도 함께 바뀌도록 수정했습니다.
- 홈 화면의 `사용 AI 목록` 제목을 `홈`으로 정리하고, 중복된 작은 `홈` 라벨을 제거했습니다.
- 상세 화면 문구를 `이 AI의 상세 사용량입니다.`로 변경했습니다.
- 공개용 저장소에서 개인 식별값과 로컬 경로가 남지 않도록 코드와 git history를 정리했습니다.
- 공개 릴리스 README를 한글 기준으로 다시 작성했습니다.

## 지원 provider

| Provider | 인증 방식 | 표시 정보 |
| --- | --- | --- |
| Codex | 로컬 Codex CLI OAuth 상태 | 세션/주간 사용률, 로컬 토큰 사용 요약 |
| Claude | Claude Code OAuth 또는 지원되는 토큰 경로 | 사용량/윈도우별 진행률, 로컬 토큰 사용 요약 |
| GitHub Copilot | `gh auth login` 기반 Copilot 접근 권한 | Copilot quota/usage 상태 |
| OpenRouter | 앱 저장 API 키 또는 `OPENROUTER_API_KEY` | 크레딧 사용률과 잔액 |

> 참고: 사용량 API는 각 vendor의 변경 가능한 응답 계약에 의존할 수 있습니다. upstream 응답이 바뀌면 provider adapter 유지보수가 필요할 수 있습니다.

## 프로젝트 구조

```text
apps/
  desktop/             # Tauri 데스크톱 앱 + React UI
packages/
  core/                # 공통 도메인 타입과 refresh orchestrator
  platform/            # 플랫폼 추상화 계약
  providers/           # provider adapter 계약과 구현
docs/
  architecture.md
  install.md
  roadmap.md
```

## 요구 사항

개발 환경 기준:

- Node.js 20 이상 권장
- npm
- Rust stable toolchain
- Tauri v2 빌드에 필요한 OS별 기본 도구
  - macOS: Xcode Command Line Tools
  - Windows: WebView2 / Visual Studio Build Tools 계열 환경

Provider별 인증을 실제로 테스트하려면 아래 로컬 상태가 필요합니다.

- Codex: 로컬 Codex CLI 로그인
- Claude: Claude Code 로그인 또는 지원되는 OAuth/token 설정
- Copilot: GitHub CLI 로그인 및 Copilot 사용 권한
- OpenRouter: OpenRouter API 키

## 빠른 시작

```bash
git clone <repository-url>
cd ai-usage-dashboard-public
npm ci
npm run typecheck
npm run build
```

개발 모드로 데스크톱 앱을 실행하려면:

```bash
npm run dev:tauri
```

## 자주 쓰는 명령

| 목적 | 명령 |
| --- | --- |
| 의존성 설치 | `npm ci` |
| 타입 체크 | `npm run typecheck` |
| 프론트엔드 빌드 | `npm run build` |
| Tauri 개발 실행 | `npm run dev:tauri` |
| macOS DMG 빌드 | `npm run build:desktop:mac` |
| Windows MSI 빌드 | `npm run build:desktop:win` |
| Rust/Tauri 체크 | `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml` |

## 설치 / 패키징

### macOS

```bash
npm run build:desktop:mac
```

생성된 DMG는 다음 경로 아래에 만들어집니다.

```text
apps/desktop/src-tauri/target/release/bundle/dmg/
```

### Windows

```bash
npm run build:desktop:win
```

생성된 MSI는 다음 경로 아래에 만들어집니다.

```text
apps/desktop/src-tauri/target/release/bundle/msi/
```

GitHub Actions의 `Release` workflow는 태그 push 또는 수동 실행으로 macOS/Windows 아티팩트를 생성하도록 구성되어 있습니다.

## 보안 / 개인정보 모델

- 이 저장소에는 실제 provider 토큰, API 키, 개인 로컬 경로를 커밋하지 않습니다.
- OpenRouter 키 등 민감한 값은 로컬 앱 저장소, OS keychain, 환경변수, 또는 사용자의 로컬 CLI 인증 상태를 통해 읽습니다.
- 공개 repo history도 개인 식별값이 남지 않도록 점검합니다.
- 단, 사용자가 로컬에서 앱을 실행하면 각 provider CLI/설정 파일을 읽을 수 있으므로, 공유 PC에서는 provider 로그인 상태와 OS 계정 권한을 주의해야 합니다.

## 공개 repo 테스트 체크리스트

공개 저장소 기준으로 다음을 통과해야 합니다.

```bash
npm ci
npm run typecheck
npm run build
cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml
```

개인정보/시크릿 점검은 최소한 아래 범주를 확인합니다.

- 개인 이름/계정명/이메일
- 사용자 홈 디렉터리를 포함한 로컬 절대 경로
- API key, token, secret, private key 문자열
- git history 내 과거 blob과 author/committer metadata

## 문서

- [Architecture](docs/architecture.md)
- [Install](docs/install.md)
- [Roadmap](docs/roadmap.md)

## 라이선스

이 공개 릴리스는 저장소에 포함된 라이선스와 각 의존성 라이선스를 따릅니다.
