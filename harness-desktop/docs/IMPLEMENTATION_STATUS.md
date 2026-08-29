# Implementation Status

현재 저장소는 모든 Phase의 핵심 경계를 보여주는 **실행 가능한 vertical prototype/foundation**이다. Phase 0–15 전체가 production 완료된 제품은 아니다. 상세 기준은 [PHASE_TRACEABILITY.md](PHASE_TRACEABILITY.md)를 따른다.

## 현재 구현

- Tauri 2 + React/TypeScript + Rust desktop foundation
- Project/Group/Conversation/Message domain과 SQLite v1 schema, CRUD, FTS5 검색, settings
- 네이티브 UI의 Project 생성, 기본 Group 생성, Conversation 생성, Message 저장·제목 갱신, FTS 검색과 재시작 복원
- Provider/Model/Capability, Task/Agent Run/Event, nullable usage, component/local model/worker schema
- deterministic routing modes, capability filtering, bounded adaptive signal
- validated Task DAG, dependency scheduling, concurrency/retry/cancel state model
- Claude Code/Codex CLI detection·auth-status와 안전한 invocation plan(stream JSON/JSONL, stdin prompt, sandbox, resume)
- M365 Graph beta boundary: required delegated permission/license/PKCE/OS-store 사전조건 모델
- Provider manifest/JSON-RPC/model catalog schema와 validation
- 3-pane workspace, conversation list, Agent Activity/Inspector, search, usage, settings, command palette의 product UI vertical slice
- architecture/security/data/provider/M365/update/worker/testing/roadmap 문서

## Prototype/Scaffold 한계

- 일반 Vite 브라우저 preview는 명시적인 demo snapshot을 사용한다. Tauri native runtime은 오류를 demo 성공으로 전환하지 않고 SQLite snapshot/CRUD/FTS 오류를 그대로 표시한다.
- native UI의 Project/Conversation/Message/Search는 persistence와 연결됐지만 group 재정렬, archive/import/export, settings 저장과 실제 Agent dispatch는 아직 end-to-end가 아니다.
- Claude/Codex는 detect와 command plan까지다. 실제 subprocess streaming parser, normalized live event, descendant cancellation, session/usage reconciliation은 외부 CLI contract test와 구현이 더 필요하다.
- M365는 configuration/capability boundary다. system-browser PKCE, secure-store token cache, HTTP/SSE client는 구현·tenant 검증이 필요하다.
- Local AI는 schema/catalog/UX 경계다. runtime/model artifact, downloader, inference, GPU backend는 포함하지 않는다.
- Component update는 inventory/policy/schema/UX 경계다. production feed, signing key, installer, atomic activation/rollback은 포함하지 않는다.
- Distributed Worker와 adaptive routing은 data/protocol/domain scaffold다. network daemon, pairing/mTLS, remote execution, 학습 모델은 포함하지 않는다.
- macOS/Windows signed packaging, notarization/code signing, accessibility/scale/security audit는 release gate로 남아 있다.

## 외부 검증 Gate

- macOS Apple Silicon 및 Windows 11 x64 clean machine
- 지원 버전의 실제 Claude Code/Codex 설치와 test account
- Microsoft 365 Copilot add-on, broad delegated consent가 허용된 test tenant와 거부 tenant
- macOS Keychain/Windows Credential Manager 또는 DPAPI
- CPU-only/Apple Metal/NVIDIA/AMD/Intel hardware matrix
- app/component/catalog signing identity와 release/update endpoint
- Phase 13용 별도 두 PC와 worker certificate lifecycle

외부 조건이 없으면 해당 Provider만 unavailable/disabled가 되어야 하며 Project/Conversation과 다른 Provider는 계속 동작해야 한다.

## 검증 명령

```sh
npm run validate:manifests
npm test
npm run test:adapter
npm run build

cd src-tauri
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

실제 완료 보고에는 위 결과와 별도로 사용한 OS/CLI/API/tenant/license/signing 조건을 기록해야 한다. mock 통과를 실 Provider acceptance로 간주하지 않는다.

현재 실행 증거는 [QA_EVIDENCE.md](QA_EVIDENCE.md)에 기록한다.
