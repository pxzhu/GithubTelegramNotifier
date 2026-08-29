# QA Evidence — 2026-08-29

이 문서는 현재 구현에서 실제로 수행한 검증과 수행하지 않은 외부 Gate를 분리한다. 목표 설계나 mock 결과를 실 Provider/production 합격으로 간주하지 않는다.

## 검증 환경

| 항목 | 값 |
|---|---|
| OS | macOS 26.4.1 (25E253), Apple Silicon arm64 |
| Node/npm | Node 22.22.1 / npm 10.9.4 |
| Rust | rustc/cargo 1.98.0 (Homebrew) |
| 관찰된 Codex CLI | 0.148.0-alpha.9 |
| Claude CLI | PATH에서 발견되지 않음 |
| Build | unsigned debug development build |

## 자동 검증

다음 명령이 성공했다.

```sh
npm run check
npm audit --audit-level=high

cd src-tauri
cargo fmt --all -- --check
cargo test --all-features
cargo clippy --all-targets --all-features -- -D warnings

npm run tauri build -- --debug --bundles app
```

관찰 결과:

- TypeScript/Vite production build 성공
- Provider manifest Vitest 4개 성공
- Generic out-of-process JSON-RPC adapter Node contract test 6개 성공
- Rust unit test 32개(macOS 기준)와 repository manifest integration test 1개 성공
- Clippy warning 0 (`-D warnings`)
- npm audit 취약점 0
- macOS debug `Harness Desktop.app` bundle 생성 성공

## Native desktop journey

빌드된 `.app`를 실행해 accessibility tree와 화면을 함께 확인했다.

1. Provider/Project가 없는 SQLite에서 native UI가 정상 시작했다.
2. `Native QA Project`를 생성했고 기본 `General` group이 실제 SQLite에 생성됐다.
3. Conversation 생성 후 user message를 보냈다.
4. 실제 Agent dispatch가 연결되지 않은 상태에서 UI가 성공을 위장하지 않고 “Message saved — provider execution is not configured yet”를 표시했다.
5. 앱을 종료·재실행한 뒤 Project, Conversation, Message가 복원됐다.
6. 첫 user message에서 만든 Conversation title이 SQLite에 유지되는 회귀 경로를 확인했다.
7. SQLite FTS5에서 저장된 `Persistent` message를 검색해 1개 결과와 Conversation 연결을 확인했다.
8. 실행 Task가 없을 때 Inspector가 가짜 진행 상태 대신 `No agent run yet`를 표시했다.
9. native timestamp는 raw RFC 3339 대신 local 표시 형식으로 렌더링됐다.
10. macOS GUI가 login-shell `PATH`를 상속하지 않는 조건에서도 표준 설치 위치의 Codex CLI를 찾았고, credential 파일을 읽지 않은 채 공식 `codex login status`로 `Connected`를 표시했다.
11. Local AI와 Component 화면은 모델/업데이트를 가짜 설치 상태로 표시하지 않고 signed catalog/update feed 미구성 Gate를 표시했다.

QA 중 생성된 데이터는 development bundle의 local app-data DB에만 존재하며 저장소나 fixture에 포함되지 않는다.

## Browser visual/interaction QA

Vite preview의 명시적인 demo adapter로 다음을 확인했다.

- 3-pane/Inspector desktop layout 및 최소 window 폭 대응
- Command Palette 열기·필터·Escape
- Search, Usage, Providers, Local AI, Components navigation
- Composer submit와 new Conversation shortcut
- console error/warning 없음
- project navigation은 native `button` semantic을 보존하고 잘못된 `listitem` role을 제거함

## 확인된 제한

- 기본 `tauri build --debug`에서 `.app` 생성은 성공했지만 DMG 단계는 이 실행 환경의 disk-image mount script에서 실패했다. `--bundles app`은 성공했다. DMG, signing, notarization은 release 환경에서 다시 검증해야 한다.
- Windows 11 x64 build/installer는 현재 macOS 호스트에서 실행하지 않았다. CI matrix는 구성했지만 remote CI 결과가 필요하다.
- Claude CLI가 없으므로 detect-unavailable 경계만 확인했다. stream/resume/usage/subagent/cancel 실계정 검증은 미수행이다.
- 관찰된 Codex CLI의 version/auth-status와 `exec resume` help/argument shape는 확인했지만 실제 모델 호출, token/usage, session resume는 실행하지 않았다.
- M365 tenant/license/consent, OS secure store, signed updater/catalog, Local Runtime/Model, 2-node Worker는 외부 Gate다.
- 실제 provider execution이 연결되기 전까지 native send는 local persistence만 수행한다.

## Release 판정

현재 결과는 executable **Developer Preview foundation** 합격 증거다. `Private Alpha`, `First Usable`, `Beta`, `Stable` 판정에는 `PRODUCT_ROADMAP.md`와 `PHASE_TRACEABILITY.md`의 추가 exit gate가 필요하다.
