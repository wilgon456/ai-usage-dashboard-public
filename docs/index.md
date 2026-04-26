# AI Usage Dashboard 문서

AI Usage Dashboard는 데스크탑에서 AI provider 사용량을 수집하고, Android 앱/위젯으로 안전한 snapshot을 동기화하는 프로젝트입니다.

## 처음 온 사람은 여기부터

1. **전체 개요를 보고 싶다면**
   - [README](../README.md)
2. **직접 설치하고 위젯까지 연결하려면**
   - [모바일 위젯 싱크 설치/세팅 가이드](widget-sync-setup.md)
3. **AI 에이전트에게 그대로 맡기려면**
   - [AI 에이전트용 설치/세팅 프롬프트](ai-agent-setup-prompt.md)
4. **Android 위젯 내부 구현이 궁금하면**
   - [Android 위젯 구현 노트](android-widget.md)

## 빠른 설치 흐름

```text
저장소 clone
  -> npm install
  -> typecheck / cargo check / Android build 확인
  -> Cloudflare Worker relay 배포
  -> Firebase Android 앱 + FCM 설정
  -> 데스크탑 앱에서 Android widget sync 켜기
  -> Android 앱에 sync URL 입력
  -> 홈 화면 위젯 추가
```

## AI 에이전트에게 던질 때

아래 링크 하나를 에이전트에게 주면 됩니다.

```text
https://wilgon456.github.io/ai-usage-dashboard-public/ai-agent-setup-prompt.html
```

에이전트에게는 다음처럼 말하면 됩니다.

```text
이 문서를 따라 AI Usage Dashboard를 설치하고 Android 위젯 sync까지 세팅해줘.
비밀 값은 출력하지 말고, Firebase/Cloudflare/provider token은 존재 여부만 확인해줘.
문제가 생기면 원인 확인 후 최소 수정으로 해결하고, 마지막에 실행한 검증 명령을 정리해줘.
```

## 공개 문서의 보안 규칙

- 실제 sync URL은 문서에 올리지 않습니다.
- `token=` 값이 들어간 URL은 비밀번호처럼 취급합니다.
- Firebase service account JSON, Cloudflare secret, provider token은 repo에 넣지 않습니다.
- 공개 문서에는 placeholder만 사용합니다.

## 문서 목록

- [모바일 위젯 싱크 설치/세팅 가이드](widget-sync-setup.md)
- [AI 에이전트용 설치/세팅 프롬프트](ai-agent-setup-prompt.md)
- [Android 위젯 구현 노트](android-widget.md)
- [설치 문서](install.md)
- [아키텍처](architecture.md)
- [로드맵](roadmap.md)
