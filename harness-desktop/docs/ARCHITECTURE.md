# 시스템 아키텍처

## 1. 목적과 범위

Harness Desktop은 하나의 채팅 UI가 아니라, 프로젝트 문맥과 사용자 정책을 기준으로 여러 AI Provider와 로컬 도구를 조정하는 local-first runtime이다. UI는 실행 의도를 제출하고 관찰하며, Core는 task graph와 권한을 관리하고, Provider Host는 vendor별 불안정성을 격리한다.

초기 지원 대상은 macOS Apple Silicon과 Windows 11 x64다. Linux Desktop과 원격 Worker는 초기 출시 범위 밖이지만 Core의 플랫폼 독립 경계를 유지한다.

## 2. 논리 구조

```text
Tauri WebView (React)
  ├─ Project / Conversation / Search
  ├─ Composer / Agent Activity / Inspector
  └─ Settings / Providers / Usage / Components
                 │ typed Tauri commands + events
Rust Application Boundary
  ├─ Project & Conversation services
  ├─ Context builder
  ├─ Router / Planner / Task DAG / Scheduler
  ├─ Permission broker
  ├─ Usage / Component / Hardware managers
  └─ Repository interfaces + event journal
                 │ Provider SPI
Provider Host
  ├─ Built-in adapters: Claude CLI, M365 HTTP, Local/OpenAI-compatible
  └─ Out-of-process adapters: JSON-RPC over stdio
                 │
Execution Targets
  ├─ vendor CLI subprocesses
  ├─ Microsoft cloud endpoints
  ├─ managed local runtime / Ollama
  └─ future paired workers
```

### 경계 규칙

- React는 DB, credential store, subprocess에 직접 접근하지 않는다.
- Tauri command는 DTO를 검증하고 application service만 호출한다.
- Domain 로직은 Tauri, 특정 CLI, HTTP SDK 타입을 참조하지 않는다.
- Provider adapter는 normalized event만 Core에 전달한다.
- Permission broker 승인 전에는 provider가 `MODIFY`, `EXTERNAL`, `DESTRUCTIVE` 작업을 실행할 수 없다.
- DB에는 secret을 저장하지 않는다. secret reference만 저장하며 실제 값은 OS secure storage에서 조회한다.

## 3. 핵심 구성요소

### Project/Conversation Manager

Project, group, conversation, message, attachment, artifact를 관리한다. Conversation은 vendor session과 동일하지 않으며 `provider_sessions`를 통해 0..N개 Provider 세션을 매핑한다. 삭제는 기본적으로 soft delete/archive이며 명시적인 영구 삭제만 실제 제거를 수행한다.

### Context Manager

입력 문맥을 `conversation`, `project`, `workspace`, `enterprise`, `task`, `agent-result` segment로 구성한다. 각 segment에는 source, sensitivity, freshness, token estimate, allowed providers가 붙는다. Provider에 전달하기 전에 data-boundary filter와 budget filter를 통과한다.

### Router

순서는 deterministic rules → optional local classifier → cloud planner다. 사용자 지정 Provider, enterprise-only context, offline mode, capability 부족, budget hard limit은 점수보다 우선하는 제약조건이다. 후보 집합을 만든 뒤 다음 개념 점수로 순위를 정한다.

```text
score = quality_fit
      - quota_pressure
      - monetary_cost
      - latency_penalty
      - failure_risk
      - data_boundary_penalty
```

결정에는 rule/version, 후보, 제외 이유, 선택 이유를 저장한다. 자동 학습은 기록만 축적하고 사용자 동의 없는 모델 학습이나 외부 전송은 하지 않는다.

### Planner와 Task Graph

Planner 출력은 자연어 실행 계획이 아니라 검증 가능한 DAG다. 각 Task는 capability, dependency, input artifact, 예상 risk, retry/fallback 정책을 가진다. 생성 시 다음 invariant를 검증한다.

- graph는 acyclic이다.
- 모든 dependency가 같은 request scope에 존재한다.
- artifact producer가 consumer보다 먼저 완료된다.
- 병렬 실행되는 write task는 workspace path가 충돌하지 않거나 격리된다.
- approval이 필요한 task는 승인 없이 `READY`가 되지 않는다.

권장 상태 전이는 다음과 같다.

```text
PENDING → BLOCKED_BY_DEPENDENCY → READY → RUNNING
                                     ├→ SUCCEEDED
                                     ├→ FAILED → RETRY_WAIT → READY
                                     ├→ CANCEL_REQUESTED → CANCELLED
                                     └→ WAITING_FOR_APPROVAL → READY/CANCELLED
```

### Scheduler

Project, Provider, 모델별 concurrency semaphore를 적용한다. cancellation token은 Task → Agent Run → subprocess/HTTP 요청으로 전파한다. retry는 동일 입력의 안전한 작업에만 자동 허용하며, 외부 변경 작업은 idempotency key 또는 사용자 재승인이 필요하다. process 종료는 graceful interrupt → 제한 시간 → forced kill 순서다.

### Provider Host

Built-in adapter와 out-of-process adapter가 같은 SPI를 구현한다. 외부 adapter는 stdin/stdout JSON-RPC를 사용하고 stdout에는 protocol frame만 허용한다. stderr는 redaction 후 diagnostic log로 저장한다. Host가 capability, protocol version, binary identity, compatibility range를 검증한 뒤 활성화한다.

### Usage Manager

usage event는 Provider의 원본 단위와 normalized nullable field를 함께 저장한다. 값이 제공되지 않으면 추정하지 않는다. 비용은 명시적인 가격표 버전과 계산 근거가 있는 경우에만 `estimated_cost`로 표시한다. quota와 token은 서로 대체하지 않는다.

### Component Manager

앱, adapter, CLI, runtime, model, plugin을 하나의 component inventory로 다룬다. 설치 출처에 따라 update executor를 선택하고, vendor/enterprise-managed 구성요소는 기본 `Notify Only`다. Harness-managed component만 atomic staging과 rollback을 보장한다.

## 4. 주요 실행 흐름

### 복합 사용자 요청

1. UI가 message와 execution policy를 제출한다.
2. DB transaction이 user message와 request envelope을 저장한다.
3. Context Manager가 허용된 문맥을 조립한다.
4. Router가 constraint와 capability를 적용해 planner를 선택한다.
5. Planner가 DAG를 만들고 validator가 검증한다.
6. Scheduler가 dependency가 없는 task를 실행한다.
7. Provider event를 normalized agent event로 append한다.
8. write/external tool은 Permission Broker에서 정지하고 승인 결과를 기록한다.
9. Reviewer/Synthesizer가 artifact와 검증 결과를 종합한다.
10. 최종 assistant message, usage, file change, terminal state를 같은 request에 연결한다.

### 복구

앱 재시작 시 `RUNNING` run은 무조건 성공/실패로 추측하지 않는다. adapter의 resume capability가 있고 provider session이 유효하면 재연결을 시도한다. 그렇지 않으면 `INTERRUPTED`로 표시하고 안전한 task만 재실행 후보로 제안한다. event sequence number를 통해 중복 frame을 제거한다.

## 5. 데이터와 이벤트

SQLite는 source of truth다. UI 상태는 projection/cache이며 재시작 후 DB에서 재구성한다. message와 agent event는 append 중심으로 저장하고, 사용자가 편집 가능한 메타데이터만 update한다. 모든 event는 다음 공통 envelope을 가진다.

```json
{
  "event_id": "uuid",
  "run_id": "uuid",
  "sequence": 42,
  "kind": "TOOL_COMPLETED",
  "occurred_at": "RFC3339",
  "provider_time": "RFC3339-or-null",
  "payload_version": 1,
  "payload": {},
  "redaction_state": "REDACTED"
}
```

UI에는 `TEXT_DELTA`, task 상태, tool name/result summary, usage, file change를 표시한다. raw chain-of-thought 수집·저장·표시는 제품 목표가 아니다.

## 6. 오류 모델

오류는 사용자 조치와 retry 판단이 가능하도록 분류한다.

| 분류 | 예 | 기본 처리 |
|---|---|---|
| `AUTH_REQUIRED` | CLI/M365 로그인 필요 | Provider 비활성, 로그인 안내 |
| `PERMISSION_DENIED` | Tenant/tool/path 권한 부족 | 해당 기능만 비활성 |
| `CAPABILITY_UNAVAILABLE` | 모델이 tool call 미지원 | 다른 후보로 route |
| `RATE_LIMITED` | quota/rate limit | `Retry-After` 존중, fallback |
| `PROTOCOL_ERROR` | 잘못된 JSON-RPC/event | adapter 격리, diagnostic |
| `EXECUTION_FAILED` | CLI non-zero/test failure | 정책 내 retry/reviewer |
| `CANCELLED` | 사용자 취소 | terminal, 자동 retry 금지 |
| `INTEGRITY_FAILURE` | hash/signature 불일치 | 설치 중단, staging 폐기 |
| `POLICY_BLOCKED` | data boundary/위험 명령 | 실행 금지, 이유 표시 |

Provider 실패는 project/conversation 데이터의 가용성에 영향을 주지 않아야 한다.

## 7. 플랫폼 추상화

Rust trait로 다음을 분리한다.

- `SecureStore`: Keychain / Windows Credential Manager 또는 DPAPI 보호 저장소
- `ProcessSupervisor`: process group/job object, signal, timeout, output framing
- `HardwareProbe`: sysctl/system APIs, optional `nvidia-smi`
- `PowerNetworkState`: metered/battery 상태(지원되는 범위만)
- `FilePermission`: canonical path, symlink 방어, platform ACL 정보
- `UpdateInstaller`: Tauri updater와 managed component atomic swap

기능 감지가 OS 이름 분기보다 우선한다. 지원되지 않는 신호는 오류가 아니라 명시적인 `unknown`으로 처리한다.

## 8. 성능 및 신뢰성 목표

초기 제품 SLO(실제 telemetry가 아닌 로컬 측정 기준):

- 10,000 conversation/수십만 message에서도 metadata 탐색과 FTS 검색 p95 300ms 목표
- 앱 cold start p95 3초 목표(Provider background probe 제외)
- UI event batching 후 streaming 갱신 60Hz 이하, perceived latency 200ms 이하 목표
- DB write는 busy timeout/WAL을 사용하며 event 유실 없이 crash recovery
- provider probe 실패가 startup을 막지 않음
- 모든 장시간 작업은 취소 가능하고 orphan subprocess를 남기지 않음

## 9. 관찰 가능성 및 개인정보

로컬 structured log에는 correlation ID, component, severity, error code만 기본 기록한다. prompt, message body, token, file content, email/Teams content는 기본 로그 금지다. 진단 bundle export는 사용자가 미리 내용을 확인하고 선택할 수 있어야 한다. 외부 telemetry와 crash upload는 opt-in이다.

## 10. 확장 규칙

새 Provider 추가가 허용되는 조건:

1. Provider SPI와 event contract를 만족한다.
2. credential ownership과 data boundary가 문서화됐다.
3. detect/auth/execute/cancel/error contract test를 통과한다.
4. capability를 과장하지 않으며 unknown usage는 `null`이다.
5. install/update source와 binary verification 정책이 있다.
6. Core의 router/scheduler에 vendor-specific branch를 추가하지 않는다.

새 실행 기능이 이 규칙을 만족하지 못하면 built-in experimental adapter로 격리하고 stable Provider API로 선언하지 않는다.
