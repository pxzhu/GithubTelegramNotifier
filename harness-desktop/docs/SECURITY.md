# Security, Threat Model, Permission Policy

## 1. 보안 목표

1. AI 출력과 repository content를 신뢰하지 않는다.
2. credential과 enterprise content가 의도하지 않은 Provider, 로그, artifact로 흐르지 않게 한다.
3. tool 실행 권한을 최소화하고, 위험·파괴 작업은 사용자의 현재 의도와 결부된 승인을 받는다.
4. adapter/runtime/model/update 공급망을 검증한다.
5. Provider 실패·침해가 Harness DB와 다른 Provider로 확산되지 않게 한다.
6. 사용자가 어떤 주체가 어떤 작업을 왜 실행했는지 확인하고 취소할 수 있게 한다.

## 2. 신뢰 경계와 자산

### 보호 자산

- local projects와 uncommitted source
- conversation/message/attachment/artifact DB
- M365 mail, Teams, SharePoint, OneDrive 기반 enterprise data
- OAuth refresh/access token과 secure-store reference
- vendor CLI session(소유자는 vendor CLI)
- code-signing/update/catalog trust root
- task approval 및 audit history
- local model/runtime와 remote worker identity

### 신뢰 경계

```text
Untrusted prompt/repository/attachment
        │
React WebView ── validated Tauri IPC ── Rust Core/Permission Broker
                                           │
               ┌───────────────────────────┼─────────────────────────┐
               │                           │                         │
        vendor CLI subprocess       M365/cloud network       local/remote adapter
          (untrusted output)        (external boundary)      (separate process/node)
```

DB와 OS secure storage도 서로 다른 경계다. DB compromise가 곧 token compromise가 되지 않아야 한다.

## 3. 위협과 통제

| 위협 | 예 | 핵심 통제 |
|---|---|---|
| Prompt injection | README가 secret 전송/명령 실행을 지시 | content는 instruction 권한을 갖지 않음, tool grant와 data boundary를 Core가 강제 |
| Confused deputy | M365 결과를 coding agent에 무단 전달 | sensitivity label과 provider allowlist, explicit context transfer 기록 |
| Path escape | `../`, symlink, junction으로 workspace 밖 접근 | canonicalization, open 시점 재검증, symlink/reparse-point 정책, allowlist |
| Command injection | 모델 문자열을 shell로 결합 | argv 기반 process spawn, shell 불필요 시 금지, command parser/allowlist |
| Destructive execution | 삭제, force push, DB wipe | 항상 per-operation 승인, target preview, broad target 거부, audit |
| Secret exfiltration | env/token이 prompt/log/error에 포함 | 최소 환경, secret redaction/canary test, provider별 data egress policy |
| Malicious adapter | protocol 위조, file/network 남용 | signed package/hash, separate process, capability/permission enforcement, path sandbox |
| Supply-chain attack | 변조된 runtime/model/catalog/update | TLS + hash + signature, signed metadata, rollback protection, trust-root rotation |
| OAuth interception | custom URI hijack/PKCE 누락 | system browser, PKCE S256, state/nonce, exact redirect, loopback random port |
| Session fixation/replay | provider/worker session 재사용 | opaque session mapping, expiry, per-run nonce/idempotency, mutual auth |
| Event spoofing | 다른 run ID 또는 duplicate sequence | Host-issued run ID, process ownership, monotonic sequence/deduplication |
| Resource exhaustion | 무한 output/subagent/download | time/output/tool/agent/disk limits, backpressure, cancellation |
| UI spoofing | AI text가 승인 dialog처럼 표시 | 승인 UI는 native/trusted chrome, provider text와 시각적 분리 |
| DB tampering/corruption | crash 중 migration/event write | transactions, FK, integrity check, backup, migration recovery |
| Remote worker lateral movement | LAN 노드가 임의 repo 접근 | explicit pairing, mTLS, capability lease, repo allowlist, revoke |
| Privacy leakage | diagnostics에 prompt/email 포함 | content-free logs default, previewable opt-in diagnostic export |

## 4. Permission 분류

| Risk | 예 | 기본 정책 | 승인 수명 |
|---|---|---|---|
| `SAFE` | 허용 workspace 내 파일 읽기, local search, git status | Project 정책이 허용하면 자동 | task 또는 project policy |
| `MODIFY` | 파일 생성/편집, formatter, test가 cache 생성 | 기본 요청 단위 승인; preview 가능 | 좁은 path/tool 범위, 해당 request |
| `EXTERNAL` | email 전송, issue 생성, cloud data 변경, publish | 항상 행위와 대상 확인 | 정확한 operation 1회 |
| `DESTRUCTIVE` | recursive delete, force push, history rewrite, uninstall/data purge | 항상 별도 강한 확인; 일부 target은 전면 금지 | 정확한 command/target 1회 |

Risk는 Provider가 낮출 수 없다. Core tool registry가 정한 값과 adapter가 보고한 값 중 더 높은 등급을 사용한다.

### 기본 정책 예시

```yaml
workspace:
  roots:
    - /canonical/project
  read: allow
  modify: ask
  follow_symlinks_outside_root: false
shell:
  enabled: true
  mode: ask-unless-readonly-allowlisted
network:
  provider_endpoints: allow
  arbitrary: ask
external_actions: always-ask
destructive_actions: always-ask
secrets:
  expose_to_provider: deny
```

실제 설정 파일 형식을 뜻하는 것이 아니라 정책 의미의 예다.

## 5. 승인 UX 요구사항

승인 화면은 다음을 포함한다.

- 요청한 Task/Agent/Provider/Model
- tool과 정확한 action
- canonical target(path, host, recipient, repository/branch)
- 변경/전송되는 데이터의 요약과 sensitivity
- 위험 등급과 되돌릴 수 있는지 여부
- command는 shell-rendered 문구가 아니라 argv와 working directory
- 허용 범위: 1회가 기본; session/project 영구 허용은 SAFE/MODIFY 일부에만 제공
- `Deny`, `Allow once`, 정책상 안전한 경우에만 좁은 범위의 `Always allow`

AI message 내부의 “승인” 버튼처럼 보이는 콘텐츠는 실제 승인으로 취급하지 않는다. 사용자 입력 event와 trusted UI nonce를 Core가 확인한다.

### 무조건 1회 승인

- 외부 recipient에게 전송/게시
- git force push, branch/tag delete, history rewrite
- recursive delete 또는 대량 overwrite
- credential/permission/security setting 변경
- component uninstall과 사용자 데이터 삭제
- 관리자 권한 상승
- workspace 외부 write

`rm -rf /`, home/workspace root 같은 광범위 target은 승인 dialog를 띄우기보다 policy로 거부한다.

## 6. Workspace와 파일 안전

1. 사용자 선택 path를 canonical absolute root로 저장한다.
2. 매 operation에서 입력 path를 root와 결합한 뒤 canonicalize한다.
3. 존재하지 않는 write target은 가장 가까운 기존 parent를 canonicalize한다.
4. symlink/junction/reparse point를 따라 root 밖으로 나가면 거부한다.
5. race 방지를 위해 가능한 플랫폼에서는 directory handle 기반 relative open을 사용한다.
6. diff와 modified file list를 run/task에 연결한다.
7. 다른 병렬 write task와 path overlap이 있으면 serialize하거나 isolated worktree를 사용한다.
8. `.git`, secret 후보, OS credential directory는 별도 rule 없이 직접 수정하지 않는다.

## 7. Subprocess 안전

- executable은 manifest 후보에서 해석한 canonical path와 identity를 기록한다.
- command 문자열을 shell에 넘기지 않고 executable + argv로 spawn한다.
- child environment는 allowlist 방식으로 구성하고 token/credential env를 자동 상속하지 않는다.
- working directory는 승인된 workspace다.
- stdout/stderr byte/time limit과 UTF-8 framing 오류 처리를 둔다.
- Unix process group, Windows Job Object로 descendant lifecycle을 소유한다.
- graceful cancel 후 bounded timeout, 마지막으로 강제 종료한다.
- 종료 code와 signal은 normalized error에 저장하되 output의 secret은 redaction한다.

## 8. Credential

### Claude/Codex 및 기타 CLI

Harness는 vendor CLI의 credential 파일, keychain entry, environment token을 읽지 않는다. 공식 auth status/실행 command만 사용하고 로그인 flow도 vendor가 소유한다. 로그아웃 시 vendor session 삭제 여부를 명확히 표시하고 기본적으로 Harness 설정만 해제한다.

### M365

desktop public client의 Authorization Code + PKCE를 사용한다. access/refresh token은 Keychain 또는 Windows 보호 저장소에 넣고 SQLite에는 account ID, tenant ID, scope metadata, secure-store reference만 둔다. 로그/diagnostic/event에 token을 넣지 않는다. logout/revoke/tenant switch와 secure-store unavailable 상태를 테스트한다.

### OpenAI-compatible endpoint

API key는 OS secure storage에 저장하고 UI에는 마지막 일부도 기본 표시하지 않는다. endpoint마다 TLS 정책과 allowed host를 저장한다. `http://localhost`는 local endpoint로만 명시적으로 허용하고 remote plaintext는 거부한다.

## 9. Data boundary

Context segment에는 다음 label 중 하나 이상을 붙인다.

- `PUBLIC`
- `PROJECT`
- `PERSONAL`
- `ENTERPRISE`
- `SECRET` (Provider 전달 금지 기본값)

Provider policy는 허용 label, 지역/endpoint, retention 특성, offline 여부를 선언한다. `ENTERPRISE` 데이터를 다른 cloud coding Provider에 넘기려면 Project 정책과 사용자 승인 또는 조직 정책이 모두 허용해야 한다. M365 응답을 자동으로 전체 conversation에 삽입하지 않고 필요한 redacted summary/reference만 전달한다.

## 10. 로그와 Audit

Audit 대상:

- Provider/Model routing 결정과 제외 이유
- permission 요청/결정/범위/actor/time
- tool 실행 시작/종료와 canonical target
- external action recipient/resource ID
- component install/update/rollback과 artifact hash/signature identity
- worker pairing/revoke/task lease
- secure store create/delete 실패(값 제외)

기본 application log에는 prompt, message, mail subject/body, file content, token, raw command output을 넣지 않는다. 사용자가 진단 bundle을 만들 때 파일 목록과 redaction preview를 제공한다.

## 11. 공급망 및 업데이트

- app/update feed/adapter/runtime/catalog는 서로 구분된 signing role을 권장한다.
- metadata에는 version, platform, size, SHA-256, expiry, signature, minimum app version을 포함한다.
- hash만 원격 metadata와 같은 서버에서 받아 신뢰하지 않고 signature root를 앱에 pin한다.
- rollback/freeze/mix-and-match 공격을 막기 위해 monotonic version과 metadata expiry를 검증한다.
- 모델은 publisher signature가 없을 수 있으므로 signed Harness catalog의 hash와 license/source를 검증한다.
- staging에서 검증/실행 smoke test 후 atomic switch한다.
- SBOM, dependency license, vulnerability scan 결과를 release evidence에 포함한다.

## 12. Remote Worker 추가 통제

Worker는 기본 비활성이다. pairing 시 양쪽에서 사람이 확인할 수 있는 짧은 인증 문자열을 표시하고 identity key를 고정한다. task는 짧은 expiry, repository/path/tool capability를 가진 lease로 제한한다. Worker는 Main의 장기 credential을 받지 않으며 cloud Provider auth는 Worker가 자체 보유하고 capability로만 광고한다.

## 13. 보안 검증 체크리스트

- [ ] IPC command가 schema/size/authorization을 검증한다.
- [ ] path traversal, Unicode normalization, symlink/junction escape test가 있다.
- [ ] argv spawn을 사용하며 shell injection fixture를 통과한다.
- [ ] secret canary가 DB/log/event/diagnostic bundle에 나타나지 않는다.
- [ ] SAFE/MODIFY/EXTERNAL/DESTRUCTIVE 각 allow/deny/revoke test가 있다.
- [ ] cancellation이 descendant process까지 종료한다.
- [ ] malformed/oversized provider event가 Host를 crash시키지 않는다.
- [ ] OAuth state/nonce/PKCE/redirect mismatch를 거부한다.
- [ ] update/model/adapter hash/signature/expiry/version failure를 거부한다.
- [ ] backup과 archive import가 zip-slip/path overwrite를 막는다.
- [ ] FTS 및 UI rendering이 untrusted HTML/script를 실행하지 않는다.
- [ ] remote worker replay/expired lease/revoked identity를 거부한다.
- [ ] dependency, secret, license, SBOM scan이 release pipeline에 있다.

## 14. Incident 대응

1. 영향을 받는 Provider/component/catalog key를 locally disable/revoke한다.
2. 진행 중 run과 worker lease를 취소한다.
3. content-free audit와 사용자가 승인한 diagnostic을 보존한다.
4. update metadata로 차단 version과 fixed version을 배포한다.
5. credential 노출 가능성이 있으면 해당 소유자(vendor CLI/Microsoft) 기준 revoke 절차를 안내한다.
6. DB는 즉시 삭제하지 않고 사용자가 export/backup한 후 repair/migrate한다.
7. post-incident ADR/threat model/test를 갱신한다.

보안 문제 제보 채널과 지원 기간은 실제 배포 전에 별도 root security policy에 공개해야 한다.
