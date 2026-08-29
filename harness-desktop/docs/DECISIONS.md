# Architecture Decision Records

상태는 `Accepted`, `Provisional`, `Superseded`를 사용한다. 결정이 바뀌면 기존 항목을 삭제하지 않고 새 ADR에서 대체 관계를 기록한다.

## ADR-001 — Tauri 2 + React + Rust Core

- 상태: Accepted
- 결정: Desktop shell은 Tauri 2, UI는 React/TypeScript, privileged runtime은 Rust로 구현한다.
- 이유: macOS/Windows native packaging, 작은 배포 크기, subprocess와 파일 접근을 Rust 경계에서 통제할 수 있다.
- 결과: WebView 차이는 E2E에서 별도 검증한다. Node API를 UI에서 직접 사용할 수 없고 사용해서도 안 된다.

## ADR-002 — SQLite를 local source of truth로 사용

- 상태: Accepted
- 결정: metadata, conversation, task/run/event, usage, component inventory는 SQLite에 저장한다. WAL, foreign key, versioned migration을 사용한다.
- 이유: local-first, transaction, FTS5, 단일 사용자 desktop 배포에 적합하다.
- 결과: 대용량 binary artifact/model은 파일 저장소에 두고 DB에는 metadata/hash/reference만 둔다.

## ADR-003 — Conversation과 Provider Session 분리

- 상태: Accepted
- 결정: Harness conversation은 provider-neutral entity이며 vendor session은 별도 mapping한다.
- 이유: 하나의 대화에서 Claude, M365, Local, 향후 Codex 실행을 혼합하고 Provider가 없어져도 기록을 보존해야 한다.
- 결과: resume 실패가 conversation 손상이나 삭제로 이어지지 않는다.

## ADR-004 — Provider/Model/Capability 분리

- 상태: Accepted
- 결정: Provider는 실행/인증/수명주기, Model은 구체적 inference 선택, Capability는 routing constraint로 모델링한다.
- 이유: vendor 이름에 고정된 router branch를 피하고 새 adapter를 Core 변경 없이 추가한다.
- 결과: capability는 versioned vocabulary이고 adapter self-report를 compatibility/catalog 정책과 교차 검증한다.

## ADR-005 — 외부 Adapter는 stdio JSON-RPC

- 상태: Accepted
- 결정: custom CLI adapter는 length 제한이 있는 newline-delimited JSON-RPC 2.0 over stdio로 연결한다.
- 이유: local socket port 관리와 방화벽 예외 없이 process ownership, cancellation, log 분리를 단순화한다.
- 결과: stdout은 protocol 전용, stderr는 diagnostic 전용이다. binary/대용량 artifact는 path 또는 content-addressed reference로 교환한다.

## ADR-006 — Event normalization과 append journal

- 상태: Accepted
- 결정: vendor stream을 normalized agent event로 변환하고 monotonic sequence와 idempotency key로 append한다.
- 이유: UI, recovery, usage, audit이 vendor payload 변화에 종속되지 않게 한다.
- 결과: 필요 최소한의 redacted raw payload만 optional diagnostic으로 보존하고 schema version을 기록한다.

## ADR-007 — 규칙 우선 Router

- 상태: Accepted
- 결정: deterministic constraint를 먼저 적용하고 optional local classifier, 마지막으로 cloud planner를 호출한다.
- 이유: 명시적 정책·data boundary를 보장하고 불필요한 비용과 quota 소비를 줄인다.
- 결과: 학습형 router는 처음부터 선택권을 갖지 않으며 Phase 14에서도 hard constraint를 우회할 수 없다.

## ADR-008 — Task와 Agent Run 분리

- 상태: Accepted
- 결정: Task는 사용자 목표의 논리 단위, Agent Run은 특정 Provider/Model에서의 실행 시도다.
- 이유: retry/fallback, multi-agent review, provider 성과 분석을 정확히 표현한다.
- 결과: 한 Task에 여러 Run이 존재할 수 있고 terminal Task 상태는 run 정책으로 계산한다.

## ADR-009 — 중앙 Permission Broker

- 상태: Accepted
- 결정: 모든 tool operation을 `SAFE`, `MODIFY`, `EXTERNAL`, `DESTRUCTIVE`로 분류하며 Core의 broker가 최종 허용을 판단한다.
- 이유: Provider 자체의 승인 UI나 sandbox만 신뢰하면 adapter마다 정책이 달라진다.
- 결과: Provider가 자체 승인 기능을 갖더라도 Harness 정책보다 권한을 넓힐 수 없다. 파괴 작업은 항상 per-operation 확인한다.

## ADR-010 — Credential ownership 유지

- 상태: Accepted
- 결정: Claude/Codex 등 CLI credential은 vendor CLI가 소유하며 Harness가 token을 읽거나 복사하지 않는다. M365 desktop credential만 OS secure storage로 관리한다.
- 이유: secret 노출면과 vendor 인증 정책 위반 가능성을 줄인다.
- 결과: CLI auth는 공식 status/실행 결과로만 감지한다. DB에는 secure-store key reference만 둔다.

## ADR-011 — Nullable usage와 출처 보존

- 상태: Accepted
- 결정: normalized usage field는 nullable이고 provider 원본 단위·수집 출처·가격표 버전을 저장한다.
- 이유: M365 등 token을 제공하지 않는 서비스에 가짜 정밀도를 만들지 않는다.
- 결과: dashboard는 `0`과 `unavailable`을 명확히 구분한다.

## ADR-012 — Local AI는 optional managed component

- 상태: Accepted
- 결정: Local AI가 없어도 모든 비-로컬 핵심 기능이 동작한다. 설치 시 llama.cpp 계열 managed runtime을 우선하되 Ollama/호환 endpoint 재사용을 지원한다.
- 이유: GPU 없는 PC와 enterprise 제한 환경에서도 앱 가용성을 유지한다.
- 결과: catalog와 artifact 검증이 없으면 자동 설치하지 않으며 모델 license 수락을 기록한다.

## ADR-013 — 설치 출처 보존 업데이트

- 상태: Accepted
- 결정: `APP_MANAGED`, `VENDOR_NATIVE`, `HOMEBREW`, `WINGET`, `SYSTEM_PACKAGE`, `MANUAL`, `EXTERNAL` 출처를 저장하고 같은 경로로만 갱신한다.
- 이유: Harness가 사용자의 package manager 또는 enterprise 관리 상태를 임의 변경하지 않게 한다.
- 결과: 외부 관리 component의 기본값은 Notify Only이고 rollback 보장은 Harness-managed 대상에 한정한다.

## ADR-014 — M365는 HTTP enterprise-knowledge Provider

- 상태: Provisional
- 결정: M365 통합은 desktop public client + Authorization Code/PKCE 및 Microsoft Graph 기반 adapter로 구현한다. 초기 capability는 enterprise retrieval/chat이며 code execution과 external action은 제공하지 않는다.
- 이유: credential을 별도 CLI façade에 위임할 이유가 없고 Graph permission/tenant 정책을 명시적으로 다뤄야 한다.
- 결과: Graph beta/preview API 의존 기능은 feature flag와 compatibility probe 아래 둔다. 실제 tenant에서 license, delegated scope, consent, endpoint 가용성을 출시 전 다시 검증한다.

## ADR-015 — 원격 Worker는 명시적 pairing 후 사용

- 상태: Accepted (future scope)
- 결정: Worker는 mutual authentication, short-lived task capability, repository allowlist를 사용한다. 자동 LAN discovery만으로 신뢰하지 않는다.
- 이유: 원격 code/file 실행은 local app보다 큰 공격면을 가진다.
- 결과: Phase 13 전까지 network listener를 기본 활성화하지 않는다. artifact는 hash와 크기를 검증한다.

## ADR-016 — Raw chain-of-thought 비수집

- 상태: Accepted
- 결정: 사용자에게 tool/event/result 수준의 관찰 가능성을 제공하고 raw chain-of-thought를 요구·저장·표시하지 않는다.
- 이유: 제품 설명 가능성에는 실행 사실과 결과가 필요하며 숨겨진 추론 전문은 필요하지 않다.
- 결과: Provider가 제공하는 내부 reasoning field는 기본 폐기하거나 합계 usage만 저장한다.

## ADR-017 — Preview 기능과 production readiness 분리

- 상태: Accepted
- 결정: preview/vendor beta API를 사용하는 adapter는 UI에 Experimental로 표시하고 stable fallback 또는 graceful disable을 갖는다.
- 이유: API·permission·SLA 변경이 앱 전체 안정성을 위협하지 않게 한다.
- 결과: mock contract 통과만으로 production-ready로 승격하지 않는다. 외부 검증 matrix의 증거가 필요하다.
