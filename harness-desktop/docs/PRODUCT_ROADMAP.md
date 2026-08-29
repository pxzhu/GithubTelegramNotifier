# Product Roadmap

## 1. 전략

15개 Phase를 번호 순서대로 모두 동시에 production화하지 않는다. 먼저 local workspace와 단일 coding Provider의 신뢰성을 만든 뒤 enterprise context, provider platform, orchestration을 확장한다. Local AI, updater, Codex, remote/adaptive 기능은 이 기반의 contract를 검증하는 순서로 추가한다.

성공 기준은 기능 개수가 아니라 다음 사용자 결과다.

1. 앱을 재시작해도 Project/Conversation/작업 상태가 안전하게 복원된다.
2. 기존 CLI 인증으로 coding task를 실행하고 취소·관찰할 수 있다.
3. M365가 허용된 tenant에서는 enterprise context를 분리된 provenance와 함께 가져온다.
4. Auto mode가 정책·capability·usage를 지키며 이유를 설명한다.
5. Provider/Local AI가 없어지거나 실패해도 local data와 나머지 기능은 유지된다.

## 2. Delivery Increment

### Increment A — Feasibility Gate (Phase 0)

목표: 가장 큰 외부 위험을 코드베이스 확장 전에 제거한다.

- macOS/Windows Tauri process/SQLite/secure-store spike
- 실제 Claude detect/auth/stream/resume/usage/cancel
- 실제 M365 public-client login과 tenant/API success/deny
- normalized event와 error fixture

Exit: test UI에서 두 Provider 각각 응답, cancel/restart, credential/log canary를 통과한다. M365 beta가 production 차단이면 제품 scope를 Retrieval/direct Graph/Experimental로 조정하는 명시적 go/no-go 결정이 필요하다.

### Increment B — Durable Desktop Workspace (Phase 1–2)

목표: Provider가 없어도 가치 있는 local-first desktop.

- Tauri/React/Rust foundation, platform abstraction
- migration/backup/error/logging/settings
- Project/group/conversation/message/search/export
- 3-pane workspace, inspector, command palette, shortcuts
- keyboard/accessibility baseline

Exit: 양 OS clean install, 수백 conversation 일상 사용, kill/restart/backup/FTS 검증.

### Increment C — Claude-first Coding Product (Phase 3)

목표: 일상 개발 작업이 가능한 첫 private alpha.

- Claude CLI detection/auth/status/version
- streaming/session/resume/cancel/usage/subagent/tool event
- workspace permission, activity, changed files
- fake CLI contract + supported real CLI matrix

Exit: representative repository에서 analyze/edit/test/review journey와 deny/cancel/recovery 통과. CLI credential 비열람 증거 확보.

### Increment D — Enterprise Context + Platform (Phase 4–6, 8)

목표: M365 context와 여러 run을 안전하게 결합하는 first usable version.

- M365 experimental/tenant-gated adapter
- Provider/Model/Capability registry
- Generic JSON-RPC Provider Host와 conformance suite
- Task/Run/Event scheduler, retry/fallback/cancel
- normalized usage/budget/dashboard

Exit: M365 허용/거부 tenant 모두 graceful, fake external adapter crash isolation, Claude usage provenance와 budget 반영.

### Increment E — Orchestration (Phase 7)

목표: Provider를 사용자가 매번 선택하지 않아도 안전하게 task를 나눈다.

- deterministic router와 routing modes
- planner/DAG/parallel execution
- reviewer/synthesizer/escalation
- decision explanation과 budget constraint

Exit: benchmark request set에서 policy violation 0, task graph/cancel/retry 안정성, 단일 Provider 대비 품질/사용량 결과 보고.

### Increment F — Local/Component Operations (Phase 9–10)

목표: optional Local AI와 안전한 component lifecycle.

- hardware probe/Ollama detection/catalog recommendation
- managed runtime/model download/license/hash/smoke/rollback
- source-aware component inventory/update policy
- signed app/adapter/runtime/catalog update

Exit: CPU-only와 대표 GPU matrix, Local 미설치 회귀, interrupted/corrupt update rollback, 양 OS signed update.

### Increment G — Extensibility Proof (Phase 11–12)

목표: Core 재설계 없이 새 CLI와 local intelligence 추가.

- Codex CLI adapter와 Claude/Codex routing/reviewer
- local router/summarizer/classifier/reviewer
- confidence/abstention와 benchmark

Exit: Codex가 Generic contract만으로 동작, Core vendor branch 최소화, local classifier가 cloud 호출을 줄이면서 품질 threshold 유지.

### Increment H — Distributed/Adaptive (Phase 13–14)

목표: opt-in 고급 사용자용 여러 PC 실행과 local routing optimization.

- paired Worker daemon, secure transport/lease/artifact
- 2-node scheduling/cancel/reconnect
- local execution outcome dataset/offline evaluation
- adaptive score with deterministic hard constraints

Exit: security review와 macOS↔Windows 2-node matrix, baseline 대비 통계적으로 의미 있는 routing 개선, privacy opt-in 결정.

### Increment I — Production Hardening (Phase 15, 전 단계 지속)

목표: 장기간 일상 사용 가능한 stable release.

- recovery/migration/backup/performance/accessibility
- dependency/supply-chain/permission/security audit
- installer/notarization/code/update signing
- support/incident/key rotation/EOL 운영

Exit: `PHASE_TRACEABILITY.md` Phase 15 evidence와 release checklist 전체 통과.

## 3. Release Milestone

| Milestone | 포함 | 제외/제한 |
|---|---|---|
| `Developer Preview` | A + UI scaffold, fake Provider | 실제 업무 데이터/서명 배포 보장 없음 |
| `Private Alpha` | B + C | Claude supported version 제한, M365 없음/실험 |
| `First Usable` | D + 기본 deterministic routing | M365 tenant-gated, Local AI optional/제한 |
| `Beta` | E + F + Codex proof | distributed/adaptive 비활성 |
| `Stable 1.0` | 검증된 A–G + Phase 15 gate | preview M365 기능은 Experimental 유지 가능 |
| `Post-1.0` | H 및 검증된 advanced local | 명시적 opt-in |

모든 Phase를 억지로 1.0에 넣지 않는다. 특히 Graph beta, distributed execution, adaptive learning은 안정성/정책 증거 없이 stable 기능으로 표시하지 않는다.

## 4. 우선순위 Backlog

### P0

- domain/provider/event/error contract와 migration 고정
- process supervisor/cancellation 양 OS spike
- Claude/M365 실제 feasibility 기록
- durable Project/Conversation/FTS
- permission broker와 secret-free logging

### P1

- Claude full adapter/activity/files/usage
- provider host/conformance와 task scheduler
- backup/recovery, search scale, accessibility baseline
- M365 feature-flag adapter와 consent/license UX

### P2

- deterministic router/DAG/reviewer
- usage/budget/Claude Saver
- component inventory/source-aware notification
- Ollama detection과 CPU-first Local AI pilot

### P3

- signed managed runtime/model/catalog/update
- Codex adapter
- local classifier/summarizer confidence
- provider SDK/adapter developer kit

### P4

- distributed worker
- adaptive routing
- broader platform/architecture support

## 5. Critical Dependency

```text
DB + event contract ──→ Task runtime ──→ Router/DAG ──→ Adaptive routing
Permission broker ────→ Claude tools ──→ Multi-agent ──→ Remote worker
Provider SPI ─────────→ Claude/M365 ───→ Codex/custom adapters
Component trust ──────→ Local runtime ─→ Signed catalog/update
Platform process/auth ────────────────→ Stable macOS/Windows release
```

UI demo를 먼저 만들 수는 있지만 dependency의 production gate를 건너뛸 수는 없다.

## 6. 제품/기술 지표

### 신뢰성

- run terminal-state completeness
- orphan subprocess count
- crash-free local sessions(opt-in 또는 local diagnostics)
- DB recovery/backup success
- first-pass/retry/fallback rate

### Routing/사용량

- Claude calls/tokens와 avoidance/escalation rate
- M365/Local delegation rate
- 평균 agents/task와 불필요 consensus 비율
- quality benchmark와 user accept/rework signal
- latency p50/p95

### UX/안전

- time-to-first-project/chat/result
- permission prompt deny/abandon/repeat rate(로컬 집계 기본)
- Provider setup success/error-actionability
- keyboard/accessibility journey pass
- policy/data-boundary violation 0

Token이나 cost가 unavailable인 Provider에는 가짜 수치를 만들지 않는다.

## 7. 주요 Risk와 완화

| Risk | 영향 | 완화/Decision Gate |
|---|---|---|
| M365 Chat API beta/광범위 scope | 초기 핵심 Provider 지연 | Experimental flag, Retrieval/direct Graph 대안, tenant validation |
| vendor CLI output/version 변화 | coding 실행 중단 | normalized adapter, golden fixture, compatibility probe, minimum/tested-through |
| cross-platform process 차이 | cancel/orphan/security | Phase 0 실기기 spike, ProcessSupervisor abstraction |
| Tauri WebView 차이 | UI regression | 양 OS E2E/accessibility, native convention adapter |
| local model 크기/GPU fragmentation | 실패/지원 비용 | catalog-driven tier, CPU fallback, optional install, smoke/rollback |
| update 공급망 | code execution compromise | role-separated signing, signed metadata/hash, source preservation |
| multi-agent 비용/충돌 | quota 낭비/코드 손상 | deterministic constraints, bounded consensus, write isolation |
| remote worker 공격면 | lateral movement/data leak | off by default, pairing/mTLS/lease, artifact-only first |
| adaptive bias/작은 표본 | 낮은 품질 routing | offline baseline, abstention, hard constraints, local-only data |
| 장기 event/FTS DB 성장 | 성능/디스크 | scale benchmark, index/projection, retention/export/backup |

## 8. Planning Cadence

- 각 increment 시작: threat/model/API/version feasibility와 exit evidence 확정
- 각 PR: 관련 test + docs + traceability 갱신
- 매 beta: supported OS/CLI/API/model/component matrix publish
- stable 후보: 외부 Provider/hardware/installer 검증과 recovery drill
- release 후: 실패/지원 데이터를 local-first 지표로 검토하고 다음 routing/catalog 기준 조정

기간 약속은 팀 규모와 외부 tenant/signing 준비가 확정된 뒤 산정한다. 외부 게이트를 일정상 “완료”로 간주하지 않는다.
