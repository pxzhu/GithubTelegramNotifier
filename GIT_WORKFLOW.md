# GIT_WORKFLOW.md

# 목적

이 문서는 Codex 및 자동화 에이전트가 이 저장소에서 작업할 때 따라야 하는 Git 및 개발 작업 규칙을 정의한다.

# AGENTS.md 연계 규칙 (필수)

이 문서는 `AGENTS.md`를 통해 강제 적용된다.

자동화 에이전트는 반드시 다음 순서로 문서를 따른다.

1. `AGENTS.md` 확인
2. `GIT_WORKFLOW.md` 절차 수행

즉, 모든 변경 작업은 `AGENTS.md`를 진입점으로 하여 본 문서 규칙에 따라 실행해야 한다.

목표

- main 브랜치는 항상 안정적인 상태 유지
- 모든 작업은 브랜치 기반으로 진행
- PR 기반 변경 관리
- 커밋 전 철저한 QA 수행
- 보안 및 개인정보 보호 검증 수행
- UI 및 사용자 경험 검증 수행

---

# 기본 작업 흐름

모든 작업은 아래 절차를 반드시 따른다.

1. main 최신화
2. 작업 브랜치 생성
3. 작업 수행
4. 코드 리팩토링
5. 보안 점검
6. 변경사항 검토
7. QA 수행 (3단계)
8. 커밋
9. 브랜치 push
10. PR 생성
11. PR 상태 대기 및 수정
12. PR merge
13. main 복귀
14. 브랜치 정리

---

# 1. main 최신화

작업 시작 전 main 브랜치를 최신 상태로 만든다.

명령

git checkout main  
git pull origin main  

설명

- 항상 최신 코드 기준으로 작업 시작
- 충돌 최소화

---

# 2. 작업 브랜치 생성

작업은 반드시 새로운 브랜치에서 진행한다.

명령

git checkout -b <type>/<scope>-<summary>

예시

feat/api-user-login  
fix/worker-duplicate-event  
docs/readme-install  
refactor/web-validation  

---

# 브랜치 타입 규칙

type 목록

feat  
fix  
docs  
refactor  
test  
chore  
security  
perf  
build  
ci  

의미

feat 기능 추가  
fix 버그 수정  
docs 문서 수정  
refactor 구조 개선  
test 테스트 추가  
chore 환경설정 변경  
security 보안 수정  
perf 성능 개선  
build 빌드 변경  
ci CI/CD 수정  

---

# scope 예시

api  
web  
worker  
auth  
docs  
readme  
deploy  
ci  

예

feat(api): add login endpoint  
fix(web): prevent double submit  

---

# 3. 작업 수행

브랜치에서 기능을 구현한다.

규칙

- 작업 범위 밖 수정 금지
- unrelated change 금지
- 하나의 브랜치는 하나의 목적만 가진다

---

# 4. 코드 리팩토링

구현이 완료되면 바로 QA 하지 않는다.

먼저 코드 품질을 개선한다.

확인 항목

- 중복 코드 제거
- 불필요한 변수 제거
- 함수 분리
- 네이밍 개선
- 코드 가독성 개선
- 불필요한 주석 제거
- 디버그 코드 제거

목표

- 유지보수 가능 코드
- 읽기 쉬운 코드

---

# 5. 보안 점검

QA 전에 반드시 보안 관련 검사를 수행한다.

확인 항목

- API 키 노출 여부
- 토큰 노출 여부
- private key 노출 여부
- secret 값 노출 여부
- DB connection string 노출 여부
- 개인정보 노출 여부

개인정보 예

- 이메일
- 전화번호
- 주소
- 주민번호
- 사용자 토큰
- 인증 정보

코드에서 아래 항목 확인

- console log에 민감정보 출력 여부
- debug 로그 제거 여부
- 환경 변수 사용 여부
- secret 하드코딩 여부

금지

- 코드에 API key 하드코딩
- access token 커밋
- 개인정보 로그 출력

---

# 6. 변경사항 검토

커밋 전에 변경 사항을 확인한다.

명령

git status  
git diff  

확인 항목

- 의도하지 않은 파일 수정 여부
- 디버그 코드 존재 여부
- 민감정보 포함 여부
- 불필요한 포맷 변경 여부

---

# QA 기본 원칙

QA는 필수 절차이다.

QA 없이 커밋하지 않는다.

QA는 반드시 다음 순서를 따른다.

1차 요구사항 검증  
2차 기능 동작 검증  
3차 회귀 테스트  

또한 QA는 아래 조건을 만족해야 한다.

- 사용자 조작 가능한 모든 UI 검증
- 웹 UI 깨짐 여부 확인
- 디자인 시스템 일관성 확인
- API 및 백엔드 검증
- 보안 영향 여부 확인

---

# QA 1차 요구사항 검증

목적

요구사항이 정확히 반영되었는지 확인

확인 항목

- 요청된 기능 구현 여부
- 요구사항 누락 여부
- 요구사항과 다른 동작 여부
- 조건 및 예외 처리 구현 여부

---

# QA 2차 기능 동작 검증

목적

기능이 실제로 정상 동작하는지 확인

확인 항목

- 정상 케이스 동작
- 실패 케이스 동작
- 반복 실행 문제 여부

---

# 사용자 조작 UI QA

사용자가 조작 가능한 모든 UI를 검증한다.

확인 대상

input  
textarea  
select  
checkbox  
radio  
toggle  
button  
link  
tab  
dropdown  
modal  
pagination  
scroll 영역  
검색창  
정렬 버튼  
필터  

검증 방법

- 클릭
- 반복 클릭
- 빠른 연속 클릭
- 다른 순서 클릭
- 값 입력
- 값 수정
- 값 삭제
- 잘못된 값 입력
- 긴 값 입력
- 빈 값 입력

예

- 버튼 클릭
- 버튼 여러번 클릭
- 뒤로가기 후 다시 클릭
- 새로고침 후 다시 실행

---

# 로그인 / 인증 QA

확인 항목

- 로그인 성공
- 로그인 실패
- 로그아웃
- 세션 유지
- 세션 만료
- 권한 없는 접근 차단

---

# 스크롤 및 화면 이동 QA

확인 항목

- 페이지 진입
- 스크롤 동작
- 뒤로가기
- 새로고침
- 페이지 재진입
- pagination 이동

---

# UI 깨짐 검사

웹 작업 시 반드시 확인한다.

확인 항목

- 레이아웃 깨짐 여부
- 버튼 위치 이상 여부
- 글자 겹침
- 모바일 레이아웃 문제
- 반응형 레이아웃 문제
- overflow 문제

---

# 디자인 일관성 검사

디자인이 기존 페이지와 일관되게 사용되는지 확인한다.

확인 항목

- 공통 버튼 스타일 사용
- 공통 폰트 사용
- 공통 spacing 사용
- 공통 색상 사용
- 디자인 시스템 준수

---

# 잘못된 데이터 입력 QA

비정상 입력을 반드시 테스트한다.

예

- 빈 값
- 특수문자
- 긴 문자열
- 잘못된 이메일
- 음수 값
- 중복 데이터

단

운영 DB를 오염시키는 입력은 금지

테스트 환경 사용

---

# QA 3차 회귀 테스트

목적

기존 기능이 깨지지 않았는지 확인

확인 항목

- 기존 페이지 정상 동작
- 기존 API 정상 응답
- 공통 컴포넌트 영향 여부
- 다른 화면 영향 여부

---

# API QA

확인 항목

- 정상 요청
- 잘못된 요청
- 필수값 누락
- 권한 오류
- 중복 요청
- 상태코드 적절성

---

# QA 체크리스트

커밋 전 확인

- 요구사항 1차 검증 완료
- 기능 동작 2차 검증 완료
- 회귀 테스트 3차 검증 완료
- 사용자 UI 조작 QA 완료
- UI 깨짐 검사 완료
- 디자인 일관성 검사 완료
- 보안 점검 완료
- 개인정보 노출 여부 확인
- git diff 검토 완료

---

# 7. 커밋

커밋 메시지 규칙

형식

type(scope): summary

예

feat(api): add employee invite  
fix(worker): prevent duplicate retry  
docs(readme): update install guide  

금지

update  
fix bug  
test  

---

# 8. 브랜치 push

명령

git push -u origin <branch-name>

---

# 9. PR 생성

PR 제목 규칙

type(scope): summary

본문 예

Summary  
변경 목적 설명

Changes  
변경 기능 목록

Validation  
수행한 QA 목록

---

# 10. PR 상태 확인

확인 항목

- CI 통과
- 리뷰 요청
- 충돌 여부

필요시 수정

git commit  
git push  

---

# 11. PR merge

기본 전략

Squash merge

이유

- 히스토리 정리
- 작은 PR 전략 유지

---

# 12. main 복귀

명령

git checkout main  
git pull origin main  

---

# 13. 브랜치 정리

git branch -d <branch-name>  
git push origin --delete <branch-name>  

---

# 금지 사항

- main 직접 수정
- main 직접 push
- QA 없이 커밋
- 개인정보 노출
- API 키 노출
- 운영 DB 오염 테스트

---

# Codex 행동 규칙

Codex는 반드시 아래 절차를 따른다.

main 최신화  
브랜치 생성  
작업  
코드 리팩토링  
보안 검사  
변경 검토  
QA 수행  
커밋  
push  
PR 생성  
PR 확인  
merge  
main 복귀  
브랜치 정리
