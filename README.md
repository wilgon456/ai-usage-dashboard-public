# AI Usage Dashboard

AI 코딩 도구 사용량을 한곳에서 확인할 수 있도록 만든 크로스플랫폼 데스크톱 대시보드입니다.

## 개요

`AI Usage Dashboard`는 Tauri 기반의 트레이 / 메뉴바 중심 데스크톱 앱입니다. 현재 다음 서비스를 대상으로 사용량과 계정 상태를 표시합니다.

- Codex
- Claude
- GitHub Copilot
- OpenRouter

브라우저 대시보드보다는 작은 시스템 트레이 / 메뉴바 앱 경험에 초점을 두고 있습니다.

## 현재 범위

- macOS 메뉴바 / 트레이 워크플로우
- Windows 시스템 트레이 패키징 대상
- Tauri v2 네이티브 셸
- React + Vite 프론트엔드
- 공통 `core` / `platform` / `providers` 패키지 구조

## 프로젝트 구조

```text
apps/
  desktop/
packages/
  core/
  platform/
  providers/
docs/
```

## 개발

1. 의존성 설치

```bash
npm install
```

2. 타입 체크

```bash
npm run typecheck
```

3. 데스크톱 앱 개발 실행

```bash
npm run dev:tauri
```

4. 프로덕션 빌드

```bash
npm run build
```

## 패키징

- macOS DMG

```bash
npm run build:desktop:mac
```

- Windows MSI

```bash
npm run build:desktop:win
```

Windows 릴리스 아티팩트는 GitHub Actions를 통해서도 생성됩니다.

## 참고

- 각 provider 연동은 해당 CLI 로그인 상태나 API 키 등 로컬 인증 상태를 기반으로 동작합니다.
- 일부 사용량 소스는 공개 고정 API가 아니라 벤더의 비공개 또는 변동 가능한 응답 계약에 의존하므로, 업스트림 변경 시 유지보수가 필요할 수 있습니다.

## 문서

- [Architecture](docs/architecture.md)
- [Install](docs/install.md)
- [Roadmap](docs/roadmap.md)
