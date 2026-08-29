# Testing Strategy

## 1. 목표

테스트는 “화면이 열린다”뿐 아니라 다음 불변조건을 증명해야 한다.

- Provider가 없어도 local data/UI가 동작한다.
- Provider/adapter 실패가 다른 run이나 DB를 손상시키지 않는다.
- 승인 없는 위험 작업과 data-boundary 위반이 실행되지 않는다.
- task/run/event/usage가 retry, cancel, crash 후에도 중복·유실되지 않는다.
- macOS와 Windows의 process/auth/secure-store/package 차이를 실제로 검증한다.

## 2. Test 계층

| 계층 | 대상 | 속도/실행 |
|---|---|---|
| Unit | domain state, router score, parser, policy, aggregation | 모든 PR |
| Property/Fuzz | DAG, event framing, path, manifest, archive/SSE | PR bounded + nightly extended |
| Repository/Migration | SQLite transaction, FTS, backup/restore | 모든 PR |
| Contract | Provider JSON-RPC, normalized event/error/usage | 모든 PR |
| Integration | fake CLI/HTTP, process cancel, component staging | 모든 PR 및 OS matrix |
| UI component | keyboard/state/error/loading/a11y | 모든 PR |
| Desktop E2E | Tauri IPC, persistence, real filesystem | merge/nightly 양 OS |
| External | Claude/Codex/M365/Local hardware | release candidate, 승인된 test account |
| Release | signed install/update/rollback | stable/beta release |

Unit test가 외부 real-provider 검증을 대체하지 않고, 외부 test가 deterministic contract suite를 대체하지 않는다.

## 3. Domain Unit

### Router

- 사용자 Provider 지정이 자동 score보다 우선
- required capability가 없는 후보 제외
- offline에서 cloud 후보 제외
- enterprise label이 허용되지 않은 Provider 제외
- budget hard limit과 concurrency/quota 제한
- Quality/Balanced/Claude Saver/Local First 비교
- 같은 input/policy/catalog revision에서 결정 재현
- 선택/제외 이유가 항상 존재
- confidence 낮음/후보 없음에서 명확히 abstain

### Task Graph/Scheduler

- cycle/self/missing dependency 거부
- independent task parallel, dependent task serialization
- parent-child terminal projection
- provider/project/model concurrency semaphore
- retry limit/backoff와 retryable 분류
- non-idempotent external action 자동 retry 금지
- cancellation race와 late event 무시
- fallback Run 1 실패 → Run 2 성공
- conflicting write path 감지

### Permission

- risk별 default와 Project override의 교집합
- Provider가 risk를 낮춰도 Core 등급 유지
- approval fingerprint 변경 시 재승인
- 1회/session/project scope expiry
- destructive/external always-ask
- broad home/workspace/root target deny
- data label/provider transfer policy

### Usage

- `null`과 0 구분
- incremental/final event 중복 방지
- provider/model/project/conversation/task/run/time aggregation
- money decimal/minor unit와 pricing revision
- Claude avoidance/delegation/retry/first-pass 지표
- unknown token/cost를 추정 표시하지 않음

## 4. DB/Migration

Matrix:

- empty DB → latest
- 지원하는 각 old schema fixture → latest
- migration step 중 process kill/disk full/constraint failure
- checksum mismatch와 newer schema 거부
- WAL crash recovery/foreign key/integrity check
- online backup 중 concurrent write와 restore
- FTS insert/update/archive/delete/rebuild
- missing Provider 상태에서 conversation/run 보존
- archive export/import ID collision과 corrupt blob

Scale fixture는 최소 10k conversation, 500k message, 수백만 event를 생성해 cold/warm search와 dashboard query p50/p95, DB 크기, migration 시간을 기록한다.

## 5. Provider Contract

모든 adapter에 공통 실행하는 black-box suite:

1. handshake identity/protocol/version
2. detect: missing, installed, permission denied, multiple binary
3. auth: connected, required, expired, cancelled
4. model/capability discovery와 unknown field
5. text/tool/subagent/usage/session event normalization
6. exactly one terminal event
7. partial/malformed/oversized/out-of-order/duplicate frame
8. cancel before/during/after terminal
9. crash/timeout/heartbeat loss와 restart
10. resume success/expired/unsupported
11. secret canary redaction
12. workspace/path/tool/network limit enforcement
13. incompatible/newer vendor version

Golden fixture에는 vendor 출력 version과 수집 날짜를 기록한다. 실제 사용자 content를 fixture에 포함하지 않는다.

## 6. CLI Process Integration

Fake CLI executable로 다음을 deterministic하게 재현한다.

- stdout JSONL chunk/partial UTF-8, stderr log, non-zero exit
- child/grandchild process 생성과 cancel
- output flood, hang, delayed terminal
- auth/version status response
- session ID와 resume
- workspace path/argv/environment capture

macOS는 process group/signal, Windows는 Job Object/CTRL event 동작을 별도 검증한다. 테스트 종료 후 orphan process scan을 수행한다.

실제 CLI release matrix는 minimum/current/newest-observed version과 로그인/비로그인 상태에서 실행한다. vendor credential 파일을 읽지 않았음을 fake home/canary와 system access tracing으로 확인한다.

## 7. M365 HTTP/Auth

Mock server:

- authorize redirect state/nonce/PKCE success와 mismatch
- user cancel, admin consent, conditional access claims challenge
- token refresh/expiry/revoke/tenant switch
- 401/403/404/429/5xx, `Retry-After`
- SSE split/reconnect/duplicate/terminal missing
- response schema additive/breaking 변화
- usage unavailable/null

Real tenant release test는 `M365_INTEGRATION.md` matrix를 따르며 test mailbox/site/team의 synthetic data만 사용한다. production employee content를 test fixture로 저장하지 않는다.

## 8. Local AI/Component Update

### Hardware/Model

- RAM/VRAM/disk unknown과 CPU-only
- Apple Unified Memory, NVIDIA/AMD/Intel profile fixture
- catalog incompatible runtime/platform/license
- low disk/metered/battery policy
- partial/resume/cancel, wrong length/hash/signature
- archive traversal/decompression bomb
- inference smoke timeout/crash/wrong model
- pinned version과 uninstall dependency

### Update

- signed valid/expired/revoked/wrong-platform metadata
- redirect/host allowlist/TLS/download interruption
- atomic activate와 crash at every step
- rollback success/failure
- source-preserving policy(Homebrew/WinGet/vendor/manual/enterprise)
- admin privilege와 external package manager가 자동 실행되지 않음
- app update + DB migration compatibility

실제 GPU/backend와 signed installer는 hardware/OS matrix에서 검증한다.

## 9. UI/UX QA

### 주요 Journey

1. Provider가 하나도 없는 clean install → Project/Chat/Settings 사용
2. Project 생성 → workspace 연결 → group/chat 생성/이동/archive/restore
3. send → routing 이유 → task progress → inspector/activity/files/usage → final result
4. approval allow/deny/cancel과 app restart recovery
5. global search/command palette/keyboard shortcut
6. Provider connect/login-required/error/update 상태
7. Local AI analyze/install 실패/rollback/ready/uninstall
8. offline 전환과 cloud Provider graceful disable

모든 input/button/toggle/tab/dropdown/modal/search/drag target을 keyboard-only로 수행한다. 반복/빠른 클릭, empty/long/Unicode/RTL input, back/refresh/reopen, narrow/wide/high-DPI, system light/dark/high contrast를 확인한다.

### Accessibility

- macOS VoiceOver, Windows Narrator
- logical focus order, visible focus, no keyboard trap
- semantic roles/labels/live region for streaming 상태
- color 외 상태 표현
- contrast, text scaling 200%, reduce motion
- drag/drop의 keyboard alternative
- progress와 error가 screen reader를 과도하게 반복하지 않음

Screenshot diff는 layout regression 보조 수단이며 semantic/accessibility test를 대체하지 않는다.

## 10. Security Test

- prompt injection이 permission/data boundary를 변경하지 못함
- path traversal, symlink/junction, Unicode normalization, TOCTOU
- argv/shell injection과 malicious filename
- HTML/Markdown/script/link rendering sanitization
- IPC unknown command/oversize payload/forged approval nonce
- token/secret canary가 DB/FTS/log/event/export/crash bundle에 없음
- malicious adapter stdout/protocol/artifact
- OAuth redirect/state/PKCE/replay
- signed metadata freeze/rollback/mix-and-match
- archive zip-slip/zip bomb/hardlink/symlink
- Worker MITM/pairing replay/expired lease/revoke

Dependency/SBOM/license/secret scan과 threat-model delta review를 CI/release gate에 포함한다.

## 11. Distributed Worker

In-memory protocol simulation 후 macOS↔Windows 실제 2-node matrix를 수행한다. pairing SAS mismatch, key revoke, clock skew, disconnect/reconnect, IP/sleep 변화, event replay, lease expiry, conflicting write, corrupt/oversized artifact, worker crash/orphan cleanup을 검증한다.

## 12. Non-functional

성능 목표는 `ARCHITECTURE.md`의 SLO를 기준으로 실제 측정 환경과 dataset seed를 함께 기록한다.

- startup/provider probe 분리
- streaming 10MB+, 수시간 task, 수천 events
- scheduler concurrency/100 task DAG
- UI memory leak과 virtualization
- DB search/dashboard under write load
- model download와 conversation 동시 사용
- suspend/resume, network transition
- 24h soak와 repeated provider crash

## 13. CI 구성

### PR

- formatting/lint/typecheck
- Rust/TS unit
- migration/repository
- provider contract/fake CLI
- bounded fuzz/property
- component UI/a11y
- secret/dependency/license checks

### Merge/Nightly

- macOS/Windows Tauri build/E2E
- extended fuzz/scale/soak
- process cancellation/orphan scan
- install/update staging tests
- latest observed CLI fixtures(non-secret test account가 있는 경우)

### Release Candidate

- signed installer/update/rollback clean VM
- real Provider/tenant/hardware matrix
- accessibility/manual UI checklist
- backup/restore/migration recovery drill
- SBOM/provenance/artifact hash archive

Flaky test는 자동 재시도로 숨기지 않고 quarantine owner/issue/expiry를 둔다.

## 14. Test Data와 Evidence

- synthetic project/mail/team/document만 사용한다.
- fixture의 vendor/API version, schema, expected normalized output을 기록한다.
- secrets는 runtime ephemeral test identity로 주입하고 artifact/log에 저장하지 않는다.
- external test evidence에는 날짜/OS/version/tenant 조건과 redacted result를 남긴다.
- release evidence는 commit/tag, test report, SBOM, signature verification, installer hash를 묶는다.

## 15. 완료 정의

기능은 다음이 모두 충족돼야 `Implemented`로 표시한다.

1. success/failure/cancel/restart 경로 코드가 있다.
2. relevant unit/integration test가 통과한다.
3. UI 상태와 접근성이 검증됐다.
4. threat model과 log/secret 영향이 검토됐다.
5. 문서/traceability가 갱신됐다.
6. 외부 dependency가 있으면 real-environment evidence가 있다.

인터페이스, mock, demo 데이터만 있는 기능은 `Scaffolded` 또는 `Prototype`이다.
