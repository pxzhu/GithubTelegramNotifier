# GitHub 전체 저장소 -> Telegram 알림 설정

이 방식은 `저장소별 워크플로`가 아니라, **GitHub App을 계정/조직의 전체 저장소에 설치**해서
하나의 Webhook 엔드포인트로 이슈/PR/커밋/머지 이벤트를 모아 텔레그램으로 보냅니다.

## 준비 파일
- Worker 코드: `github-global-telegram-worker.js`

## 1) 텔레그램 봇 준비
1. 텔레그램에서 `@BotFather`로 봇 생성
2. `TELEGRAM_BOT_TOKEN` 확보
3. 알림 받을 채팅의 `chat_id` 확보
   - 개인: 봇과 대화 후 `getUpdates`에서 `message.chat.id` 확인
   - 그룹: 그룹에 봇 추가 후 메시지 전송, `chat.id` 확인(보통 `-100...`)

## 2) Webhook 수신기 배포 (Cloudflare Worker 예시)
1. Worker 프로젝트 생성 후 `github-global-telegram-worker.js`를 메인 코드로 사용
2. 아래 환경값 설정
   - Secret: `GITHUB_WEBHOOK_SECRET`
   - Secret: `TELEGRAM_BOT_TOKEN`
   - Variable/Secret: `TELEGRAM_CHAT_ID`
3. 배포 후 HTTPS URL 확보 (예: `https://github-telegram.example.workers.dev`)

권장: `GITHUB_WEBHOOK_SECRET`는 충분히 긴 랜덤 문자열 사용

## 3) GitHub App 생성
GitHub `Settings -> Developer settings -> GitHub Apps -> New GitHub App`

필수 설정:
1. **Webhook URL**: Worker URL
2. **Webhook secret**: Worker에 넣은 `GITHUB_WEBHOOK_SECRET`와 동일
3. **Repository permissions**
   - Issues: Read-only
   - Pull requests: Read-only
   - Contents: Read-only (push payload 확인 목적)
   - Metadata: Read-only
4. **Subscribe to events**
   - Issues
   - Pull request
   - Push

## 4) 전체 저장소 설치
1. 생성한 GitHub App의 `Install App` 클릭
2. 대상 계정/조직 선택
3. Repository access에서 **All repositories** 선택

이렇게 하면 현재 저장소 + 새로 생기는 저장소까지 같은 App으로 이벤트를 받습니다.

## 5) 동작 확인
1. 아무 저장소에서 이슈 생성/수정/종료
2. PR 생성/업데이트/머지
3. 커밋 푸시
4. 텔레그램 수신 확인

## 이벤트 범위/필터
기본 포함 이벤트는 `issues,pull_request,push` 입니다.  
Worker 환경변수 `INCLUDE_EVENTS`(쉼표 구분)로 조절할 수 있습니다.

예시:
- `INCLUDE_EVENTS=issues,pull_request,push`
- `INCLUDE_EVENTS=pull_request,push`

## 참고
- 개인 계정/조직 전체 저장소를 안정적으로 커버하려면 GitHub App 방식이 가장 깔끔합니다.
- 기존 repo별 GitHub Actions 방식과 함께 쓰면 중복 알림이 생길 수 있습니다.
