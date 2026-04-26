# AI Usage Dashboard — 한국어 현황 정리

최종 업데이트: 2026-04-23

이 문서는 한국어 UI를 기준으로 **사용 방법 / 현재 작업 사항 / 남은 작업 사항** 을 한 번에 훑을 수 있도록 정리한 요약본입니다. 세부 설계 문서는 [`current-status.md`](./current-status.md), [`architecture.md`](./architecture.md), [`roadmap.md`](./roadmap.md) 를 참고하세요.

---

## 1. 사용 방법

### 1-1. 설치

- **macOS**: 릴리스의 `AI Usage Dashboard_0.1.0_aarch64.dmg` 를 내려받아 Applications 에 드래그합니다. 최초 실행은 우클릭 → **열기** 로 Gatekeeper 경고를 통과시키세요. 앱은 메뉴바에 상주합니다.
- **Windows**: 릴리스의 `AI Usage Dashboard_0.1.0_x64_en-US.msi` 를 실행합니다. SmartScreen 이 뜨면 **추가 정보 → 실행** 순서로 진행하세요. 시스템 트레이에서 확인할 수 있습니다.

자세한 내용은 [`install.md`](./install.md) 참고.

### 1-2. 프로바이더 연결

앱 내 **Settings → Providers** 에서 상태를 확인하거나, 홈 카드의 "연결 방법" 버튼으로 바로 가이드를 열 수 있습니다.

| 프로바이더 | 인증 방식 | 준비물 |
| --- | --- | --- |
| Codex | CLI 로그인 | `codex login` 실행 → `~/.codex/auth.json` 생성 |
| Claude | CLI 로그인 + 키체인 | `claude` CLI 로그인 (keychain / `~/.claude/.credentials.json`) |
| Copilot | GitHub CLI | `gh auth login` 후 Copilot 사용 권한 필요 |
| OpenRouter | API 키 | `OPENROUTER_API_KEY` 환경 변수 또는 설정 모달에서 저장 |

macOS 에서는 최초 1회 키체인 접근 승인이 필요합니다. 이후에는 앱 내 캐시를 통해 반복 프롬프트가 뜨지 않습니다.

### 1-3. 메뉴바 / 트레이 동작

- 트레이 아이콘은 항상 흰색으로 렌더링되어 라이트/다크 메뉴바 모두에서 가독성이 유지됩니다.
- 기본 트레이 대상은 **마지막으로 본 AI** 입니다. 홈 또는 상세 화면에서 마지막으로 열람한 프로바이더를 따라 아이콘과 요약 수치가 바뀝니다.
- 설정에서 `최고 사용률 AI` 또는 특정 AI 고정을 선택할 수 있습니다.
- 트레이 아이콘 클릭: 패널 토글 (macOS 는 트레이 아래, Windows 는 커서 근처로 자동 배치).
- 트레이 메뉴: `대시보드 열기`, `설정 열기`, `종료`.

### 1-4. 홈 / 상세 화면

- 홈은 기본으로 **컴팩트 뷰** 로 열립니다. 연결된 프로바이더만 보이고, 설정의 `숨김/비활성 표시` 옵션으로 전체 목록도 볼 수 있습니다.
- 각 카드의 퍼센티지는 설정의 `표시 방식` (`used` / `left`) 을 따릅니다.
- 연결되지 않은 프로바이더 카드는 인라인으로 "연결 방법" 버튼을 노출합니다.

### 1-5. 알림

- 1차 / 2차 임계치를 각각 퍼센트 단위로 설정할 수 있습니다.
- 임계치 돌파 시 데스크톱 알림이 한 번씩 발송되며, 사용량이 임계치 아래로 다시 내려가면 다시 알림을 발송합니다.

---

## 2. 현재 작업 사항 (2026-04-22)

### 2-1. 완료된 주요 페이즈

- **Phase 11** — 임계치 알림 + 동적 트레이 아이콘.
- **Phase 12** — 라이트/다크 토큰 완전 분기 및 브랜드 컬러 유지.
- **Phase 13** — 프로바이더별 연결 가이드, 상태 머신 기반 카드, 홈 컴팩트 모드 지속화.
- **Phase 14** — 네이티브 자격증명 레지스트리, 시작 시 키체인 워밍업, Codex/Claude 토큰 자동 갱신.
- **Phase 15** — 라이트 모드 토큰 마감, `ko`/`en` 지역화 인프라, 접근성 보강, 트레이 위치/메뉴 마감.
- **Phase 18** — 트레이 첫 프레임 흰색 마스킹, Claude 진단 로그, 사이드 네비/홈/푸터 폴리시.
- **Phase 17** — Settings 정리, 자동 플랫폼 감지, Switch 대비 개선, Home/상세 연결 CTA 재배치.

### 2-2. 최근 UI/UX 정리 (본 커밋 세트)

시각 검증 후 아래 항목을 일괄 반영했습니다.

- Phase 18: 앱 시작 직후 트레이 아이콘도 Rust 쪽에서 흰색 마스크를 적용해 첫 프레임 검은 아이콘 플래시를 제거.
- Phase 18: Claude 사용량이 비어 보일 때 `AI_USAGE_DEBUG_CLAUDE=1 npm run dev:tauri` 로 실행하면 응답 상태 / content-type / 본문 프리뷰가 stderr 에 찍히도록 진단 로그 추가. 이 값을 공유하면 후속 phase 에서 endpoint / UA 를 맞출 수 있습니다.
- Phase 18: 사이드바 홈 아이콘을 House 로 교체하고, `?` 버튼은 GitHub issues 페이지를 기본 브라우저로 엽니다.
- Phase 18: 홈 카피를 `사용 AI 목록` 기준으로 정리하고, 푸터는 `새로고침` 버튼과 마지막 업데이트 라벨을 하나의 액션으로 통합.

- 트레이 아이콘 / AI 로고 순백색 강제 (템플릿 모드 해제 + 캔버스 `source-in` 플랫닝).
- 기본 트레이 대상을 `max` → `last-viewed` 로 변경하고, 기존 로컬스토리지에 저장된 `"max"` 값을 마이그레이션.
- `homeCompactView` 기본값 `true` 로 변경.
- macOS 키체인 접근을 메모리 캐시로 감싸 반복 프롬프트 제거 (`save/clear` 시 캐시 프라임/무효화).
- 설정에서 효과가 없던 `Menubar Icon` 행 제거.
- 알림 임계치 행을 `1차 알림` / `2차 알림` 으로 분리해 레이아웃 파손 해결.
- 카드 컴팩트 요약이 `displayMode` 를 존중하도록 수정 (사용률 / 잔여율 표시 + 라벨 토글).
- 미연결 프로바이더 카드에 "연결 방법" 인라인 CTA + 모달 가이드 연결.
- 카드 / 탭 / 설정 / 모달 / 상대시간 포맷터 / 트레이 메뉴 **전역 한국어 번역**.
- 버튼 / 탭 / Segmented / SideNav 에 `cursor-pointer` 적용, 비활성 버튼은 `not-allowed`.
- 브라우저 스토리지에서 레거시 데모 자격증명 prune.

### 2-3. 검증

- `pnpm run typecheck`
- `cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml`
- `cargo clippy --all-targets -- -D warnings`
- 데스크톱 앱을 macOS에서 띄워 트레이 / 홈 / 설정 / 연결 모달 육안 확인

### 2-4. 자격증명 & 보안 완료

- 크로스 플랫폼 자격증명 저장소 추상화: `CredentialSource` trait + registry 로 Tauri 커맨드의 자격증명 해석 경로를 통합.
- 프로세스 시작 시 키체인 캐시 워밍업을 백그라운드에서 수행해 첫 새로고침 클릭 시 반복 프롬프트를 줄임.
- Codex / Claude 의 만료된 OAuth 액세스 토큰을 자동 갱신하고, 회전된 토큰을 원본 저장소에 다시 기록.

### 2-5. UI / UX 완료

- 트레이 패널 포지셔닝을 메뉴바 아이콘 기준 수평 중앙 정렬로 보정하고, 포지션 커밋 뒤 한 프레임 지연 포커스로 깜빡임을 줄였습니다.
- 라이트 모드에서 `danger / warn / good` 토큰과 포커스 링 대비를 마감했습니다.
- 문자열 지역화 인프라를 추가해 `ko` / `en` 번들과 언어 토글, 트레이 메뉴 라벨 동기화를 연결했습니다.
- 버튼 / 탭 / Segmented / Switch / Progress / 모달 / 카드 CTA 에 `aria-*`, 키보드 포커스, 가시적 포커스 링을 보강했습니다.

### 2-6. 데이터 / 안정성 완료

- Claude 사용량 파서를 동적 윈도 스키마로 바꿔 키 이름이 바뀌어도 `utilization` / `resets_at` 형태면 카드에 진행률 라인이 표시되도록 했습니다.
- 네 프로바이더 모두에 공통 fetch 상태를 추가해 TTL 캐시, 네트워크 / 5xx 지수 백오프(1→2→4→8→16→30초), 수동 새로고침 강제 우회를 적용했습니다.
- Codex / Claude 에서 알려진 사용량 필드가 전혀 없으면 빈 카드 대신 스키마 드리프트 오류를 표시하도록 가드했습니다.
- Claude / Codex / Copilot 연결 모달의 기본 액션이 터미널에서 `claude auth login`, `codex login`, `gh auth login --web` 를 직접 실행하도록 바뀌었습니다.
- 홈 화면은 항상 컴팩트로 고정되고, `리셋 타이머` 설정에는 상대/절대 표시 차이를 설명하는 문구를 추가했습니다.

---

## 3. 남은 작업 사항

우선순위 순으로 정리합니다.

### 3-1. 패키징 & 배포

1. 샌드박스 / AppleScript 제한 환경에서도 동작하는 DMG 번들링 파이프라인.
2. Windows Rust 타겟을 갖춘 호스트에서 MSI 크로스 빌드 검증.
3. macOS 공증(Notarization) & 코드 서명 적용.

### 3-2. 자동화 & 출시

1. GitHub Actions 릴리스 워크플로우의 macOS 서명 / 공증 단계 결합.
2. `dev-tools/` 의 로컬 실행 스크립트 문서화.
3. 간단한 온보딩 튜토리얼 (최초 실행 시 프로바이더 연결 체크리스트).

---

## 4. 관련 문서

- [`current-status.md`](./current-status.md) — 영문 상세 상태.
- [`install.md`](./install.md) — 플랫폼별 설치 / 인증 절차.
- [`roadmap.md`](./roadmap.md) — 로드맵.
- [`architecture.md`](./architecture.md) — 패키지/런타임 구조.
- `phase-*-plan.md` — 각 페이즈의 설계 의도.
