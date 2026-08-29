# Phase 0–15 추적표

## 1. 읽는 법

이 표는 126개 제품 요구사항을 구현 단위로 연결한다. 현재 저장소는 전체 제품의 **vertical prototype/scaffold**이며, 모든 Phase를 production-ready로 완료했다는 뜻이 아니다. 외부 계정이나 실기기가 필요한 항목은 자동 테스트와 별개로 `External-gated` 상태를 유지한다.

증거 등급:

- `A`: 자동 테스트 또는 build에서 검증
- `M`: mock/demo UI에서 수동 검증 가능
- `C`: contract/interface와 문서만 존재
- `E`: 외부 환경 검증 필요
- `—`: 미구현

## 2. 요약

| Phase | 목표 | 현재 범위 | 증거 | Production 완료를 막는 항목 |
|---:|---|---|---|---|
| 0 | Architecture & Feasibility | Provider/DB/process/security 구조와 probe 경계 | C | macOS/Windows 실기기, Claude 계정, M365 tenant 검증 |
| 1 | Application Foundation | Tauri/React/Rust 앱, macOS debug `.app`, desktop workspace shell | A/M/E | Windows와 양 OS signed installer, 장기 crash/성능 검증 |
| 2 | Project & Conversation | native project/group/conversation/message persistence, FTS/search UI | A/M | archive/import/export, DnD/accessibility, 대규모 DB/FTS 완성 |
| 3 | Claude Code | CLI adapter/probe/event normalization 설계 및 실행 경계 | C/E | 공식 CLI 실계정으로 auth/stream/resume/usage/subagent matrix |
| 4 | M365 Copilot | HTTP provider/auth 상태 및 enterprise capability 경계 | C/E | tenant consent/license, Graph beta API 실제 호출과 정책 승인 |
| 5 | Provider Platform | provider/model/capability abstraction과 manifest/protocol | A/C | 제3자 adapter compatibility suite와 signature/trust 배포 |
| 6 | Agent Runtime | task/run/event, scheduler/retry/cancel 모델 | A/M | 실제 subprocess cancellation/crash recovery/concurrency soak |
| 7 | Auto Routing | deterministic/capability/budget mode 및 DAG 시연 | A/M | 실제 planner 품질 평가, conflicting write isolation |
| 8 | Usage & Budget | nullable normalized usage와 dashboard 시연 | A/M | provider별 실측 parser, quota API/가격표 운영 절차 |
| 9 | Local AI Manager | hardware/catalog/install 상태 모델과 UX | A/M/E | signed catalog/binary hosting, GPU/CPU matrix, license UX |
| 10 | Component Updates | inventory/policy/compatibility/rollback 상태 모델 | A/M/E | production signing keys, Tauri endpoints, package manager 실기기 |
| 11 | Future CLI/Codex | generic CLI 경계로 Codex 추가 가능한 구조 | C/E | Codex 실계정 JSONL/resume/usage/version 호환성 |
| 12 | Advanced Local Intelligence | local classifier/reviewer를 꽂을 router 경계 | C | 실제 모델 선정, benchmark, confidence calibration |
| 13 | Distributed Worker | pairing/protocol/security 설계 | C | network daemon, mTLS/pairing 구현, 2-node 실험 |
| 14 | Adaptive Routing | 학습용 실행 결과 schema/score 설계 | C | 데이터 충분성, offline evaluation, privacy/opt-in 결정 |
| 15 | Production Hardening | 보안/QA/release gate 정의 | C | audit, signing/notarization, backup recovery, accessibility |

## 3. Phase별 상세 추적

### Phase 0 — Architecture & Feasibility

| 요구 | 저장소에서 확인할 범위 | 완료 증거 | 외부 게이트/합격 조건 |
|---|---|---|---|
| Tauri macOS/Windows 실행 | Tauri 2 config와 platform-neutral Core | A/C | macOS Apple Silicon 및 Windows 11 x64 clean VM 실행 녹화/로그 |
| SQLite/FTS5 | bundled SQLite, migration/search repository | A | migration up/down 대신 forward recovery, 100k+ message benchmark |
| Claude detect/auth/version | command allowlist 기반 probe | C/E | `claude` 공식 설치별 path, auth status, 비로그인 상태 검증 |
| Claude stream/resume/usage | stream-json normalization contract | C/E | 실제 CLI 버전 matrix와 golden JSONL fixture |
| M365 login/Copilot | PKCE + secure-store 경계 | C/E | 회사 tenant delegated consent와 preview endpoint 성공/거부 시나리오 |
| secure credential store | `SecureStore` 추상화/DB secret 금지 | C/E | Keychain/Windows 실제 round-trip 및 locked-session 동작 |
| subprocess cancellation | supervisor 설계 | C/E | Unix process group, Windows Job Object descendant 종료 테스트 |

Exit gate는 한 test UI에서 Claude와 M365 각각 실제 응답을 받고, credential/log 누출 없이 cancel/restart까지 검증하는 것이다. 현재 외부 게이트가 남아 있으므로 Phase 0은 production 완료가 아니다.

### Phase 1 — Application Foundation

| 요구 | 현재 범위 | 남은 검증 |
|---|---|---|
| Tauri/React/Rust | desktop shell과 typed bridge | production build on both OS |
| settings/log/error/platform | 도메인 타입과 UI 상태, 오류 경계 설계 | durable settings, redacted log bundle, OS-specific failure tests |
| HIG/Fluent layout | 3-pane + optional inspector productivity UI | native menu/context menu, high contrast, 200% scale |
| packaging | config/scaffold | signed DMG/MSIX 또는 installer, upgrade/uninstall |

### Phase 2 — Project & Conversation

| 요구 | 현재 범위 | 남은 검증 |
|---|---|---|
| Project/group/conversation/message | native 생성/목록/메시지 저장과 첫 user message 제목 갱신 | update/delete/archive/restore, ordering transaction |
| persistence | SQLite v1, WAL, foreign key, native app 재시작 복원 | kill/disk-full durability, backup/corruption recovery |
| FTS search | FTS5 trigger와 native 검색→Conversation 이동 | tokenizer(locale), ranking, rebuild, 100k+ corpus |
| DnD/favorite/tag/archive | 대표 UI state | keyboard alternative, cross-group/project transaction |
| export/import | archive format 설계 대상 | versioned Markdown/JSON/archive round-trip, unsafe path rejection |

### Phase 3 — Claude Code Integration

필요 adapter contract: detect → version → auth status → execute stream → normalized events → session mapping/resume → usage → cancel. 실제 CLI가 없는 CI에서는 golden JSONL fixture로 parser를 검증하되, 이는 real account acceptance를 대체하지 않는다. CLI credential 파일을 열거나 복사하는 구현은 금지한다.

외부 matrix:

- macOS/Windows, 로그인/비로그인/만료
- 지원 최소·현재·새 버전
- 정상 stream, tool call, subagent, malformed/partial line
- resume 가능한 session과 삭제/만료 session
- Ctrl-C graceful cancel과 descendant cleanup
- usage 제공/누락 event

### Phase 4 — M365 Copilot Integration

desktop public client의 system browser + Authorization Code/PKCE를 사용한다. 현재 알려진 Copilot Chat API 기능은 preview/Graph beta 의존성이 있으므로 feature flag가 필요하다. work/school delegated permission, license, admin consent, tenant 정책을 실제 환경에서 확인한다. application-only 권한, arbitrary actions, code execution을 초기 capability로 선언하지 않는다. 자세한 내용은 `M365_INTEGRATION.md`에 둔다.

### Phase 5 — Provider Platform

Provider contract의 안정화 단위는 vendor SDK가 아니라 normalized request/event/error다. 합격 조건:

1. built-in mock adapter와 최소 1개 out-of-process sample adapter가 같은 contract suite를 통과한다.
2. manifest의 protocol range, OS/architecture, capability, install/auth/update strategy를 검증한다.
3. unknown field는 호환 가능하게 무시하되 required semantic 변화는 major protocol을 올린다.
4. adapter crash가 Host/Core/UI를 중단하지 않는다.
5. capability revoke가 기존 conversation/session을 삭제하지 않는다.

### Phase 6 — Agent Runtime

Task/Run/Event 분리, DAG dependency, concurrency, retry/fallback, cancellation, parent-child activity가 범위다. 자동 retry는 read-only/idempotent 작업에 제한한다. 합격 시나리오는 동일 Task의 Run 1 실패 → 다른 Provider Run 2 성공, parent 취소 전파, crash 후 `INTERRUPTED` 복구, event 중복 제거를 포함한다.

### Phase 7 — Auto Routing & Multi-Agent

`Quality First`, `Balanced`, `Claude Saver`, `Local First`, `Custom`을 정책 입력으로 취급한다. Provider 직접 선택은 자동 점수보다 우선한다. 합격 조건은 capability hard constraint, budget hard limit, enterprise boundary, independent task parallelism, reviewer escalation, 선택 이유 표시다. consensus는 비용 상한과 사용자 정책이 있는 경우만 활성화한다.

### Phase 8 — Usage & Budget

provider/model/project/conversation/task/run/time range별 nullable usage를 집계한다. `0`, `unknown`, `not applicable`을 구분한다. Claude Avoidance와 delegation 지표는 task routing 기록에서 계산하고 token을 임의 역산하지 않는다. 실제 parser는 provider/version별 fixture와 reconciliation test가 필요하다.

### Phase 9 — Local AI Manager

hardware probe → signed catalog → compatibility 추천 → disk/license 확인 → staged download → hash/signature 검증 → smoke inference → atomic register 흐름을 따른다. 설치 실패 시 기존 runtime/model을 보존한다. CPU-only는 정상 tier이며 Local AI 미설치는 정상 상태다. GPU/driver별 실기기 검증 없이는 추천을 production으로 승격하지 않는다.

### Phase 10 — Component Update Manager

install source를 보존하고 안전한 Harness-managed component만 `Update All` 대상에 자동 포함한다. 관리자 권한, package manager, enterprise-managed 항목은 확인/알림만 제공한다. production exit gate에는 signed app feed, downgrade 방지, offline/partial download, rollback, revoked key/catalog, macOS notarization, Windows signing 테스트가 포함된다.

### Phase 11 — Future CLI Agents

Codex adapter는 Generic CLI contract 검증의 첫 사례다. 공식 OpenAI documentation에 명시된 `codex login status`, `codex exec --json` JSONL, `codex exec resume` interface를 사용하되 출시 시점 버전으로 다시 확인한다. 기존 로그인 session을 이용하고 auth token 파일을 읽지 않는다. Claude/Codex 상호 fallback과 동일 task reviewer 교차 실행이 Core vendor branch 없이 동작해야 완료다. [Codex CLI command reference](https://developers.openai.com/codex/cli/reference)

### Phase 12 — Advanced Local Intelligence

Local router/summarizer/reviewer/classifier는 동일 Local Provider SPI를 사용한다. 모델 선정은 catalog-driven이고 품질 기준은 intent macro-F1, abstention calibration, summary faithfulness, latency/memory다. confidence가 낮으면 deterministic/cloud 단계로 넘기며 local 결과만으로 위험 권한을 넓히지 않는다.

### Phase 13 — Distributed Worker

현재는 프로토콜/보안 설계 범위다. 완료에는 2대 이상의 PC pairing, mutual authentication, capability lease, remote cancel, chunked artifact integrity, disconnect/reconnect, version skew, repository allowlist, worker revoke가 필요하다. 자세한 내용은 `DISTRIBUTED_WORKERS.md`를 따른다.

### Phase 14 — Adaptive Routing

Task type/complexity/provider/model/latency/usage/success/retry/user acceptance를 로컬에 기록한다. 학습 전 deterministic baseline과 offline replay set을 고정한다. adaptive score는 hard policy를 우회할 수 없고, 충분하지 않은 표본에서는 abstain한다. 사용자 콘텐츠의 외부 학습 전송은 별도 opt-in 없이는 금지한다.

### Phase 15 — Production Hardening

다음 증거가 모두 있어야 완료다.

- crash/restart, DB backup/restore와 migration failure recovery drill
- threat model review, dependency/SBOM/secret scan, penetration test
- binary/model/catalog signature 및 download path hardening
- macOS code signing/notarization, Windows code signing/installer reputation 계획
- update key rotation/revocation/rollback drill
- 대규모 DB, long stream, concurrency, memory/CPU 성능 기준
- VoiceOver/Narrator, keyboard-only, contrast, reduce motion, 200% scale
- clean install/upgrade/uninstall/offline/managed-device matrix

## 4. Acceptance Criteria 매핑

원 제품 명세의 25개 핵심 기준은 다음 검증 묶음으로 관리한다.

| 기준 | 담당 Phase | 자동화 | 외부/수동 |
|---|---:|---|---|
| macOS/Windows 실행 | 0,1,15 | build matrix | signed clean install |
| Local LLM 없이 동작 | 1,9 | no-local provider suite | clean device smoke |
| CLI 기존 인증 재사용/credential 비열람 | 3,11 | fake-home/secret scan | real login matrix |
| M365 enterprise context | 4 | HTTP/auth contract mock | licensed tenant |
| Project/group/history/search | 2 | repository/FTS/restart tests | keyboard/UI QA |
| Provider/model/run 관찰성 | 3,5,6 | event contract | real provider stream |
| Usage 집계 | 8 | nullable aggregation | provider reconciliation |
| Provider 제거 시 데이터 보존 | 2,5 | orphan mapping migration | restore drill |
| Local hardware/CPU 추천/무결성 | 9 | catalog/hash/failure tests | hardware matrix |
| 외부 CLI 확장/Codex | 5,11 | adapter contract | real CLI version matrix |
| component 상태/안전 update | 10,15 | policy/rollback tests | signing/package managers |
| task 분해/병렬 실행 | 6,7 | DAG/property/concurrency tests | workspace conflict UX |

## 5. 상태 갱신 규칙

- PR에서 기능을 추가하면 해당 행의 증거 링크와 test 이름을 추가한다.
- mock만 통과한 항목을 `Implemented`로 올리지 않는다.
- 외부 검증은 날짜, OS/CLI/API 버전, tenant/license 조건, 결과 artifact를 release evidence에 남긴다.
- preview API가 stable로 전환되거나 폐기되면 ADR과 M365 문서를 먼저 갱신한다.
- Phase 완료 선언은 코드 존재가 아니라 명시된 exit gate 통과를 기준으로 한다.
