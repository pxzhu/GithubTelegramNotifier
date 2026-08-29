# Release, Signing, Component Update Plan

## 1. 목적

Harness Desktop 자체와 Provider adapter, CLI agent, local runtime, local model을 한 화면에서 관리하되 소유권과 설치 출처를 침범하지 않는다. “최신”은 version 문자열 비교만이 아니라 signature, compatibility, policy, 실제 probe가 모두 유효한 상태를 뜻한다.

## 2. Release channel

| Channel | 대상 | 정책 |
|---|---|---|
| `stable` | 일반 사용자 | production gate와 staged rollout 통과 |
| `beta` | opt-in 사용자 | 기능 완성, preview Provider 허용, downgrade 안내 |
| `nightly` | 개발/QA | commit 기반, 데이터 호환성 보장 제한, 별도 data directory 권장 |

channel 변경 시 version downgrade와 DB schema compatibility를 검사한다. stable data directory를 nightly가 자동 공유하지 않는다.

Semantic Versioning을 앱/API/adapter에 적용한다. Provider manifest schema, JSON-RPC protocol, DB schema, archive schema는 app version과 독립된 version을 갖는다.

## 3. Build Matrix

최소 release artifact:

- macOS Apple Silicon (`aarch64-apple-darwin`)
- Windows 11 x64 (`x86_64-pc-windows-msvc`)

향후 Intel macOS/Windows ARM64는 별도 지원 선언 후 추가한다. universal binary를 만들 경우 두 architecture artifact를 각각 재현·검증한다.

CI stage:

```text
source checkout at immutable commit
  → lockfile/dependency policy + secret/license/vulnerability scan
  → TS lint/type/test/build
  → Rust fmt/clippy/test/build
  → provider contract/security/migration tests
  → platform Tauri bundle
  → artifact malware/signing preflight
  → code sign
  → macOS notarize/staple / Windows signature verify
  → updater package sign
  → SBOM + provenance + checksums
  → isolated clean-VM install/upgrade/smoke
  → signed channel metadata publish
```

서명 전 artifact와 서명 후 artifact hash를 모두 provenance에 기록한다. CI가 production signing key 원문을 읽을 수 없도록 HSM/managed signing service 또는 짧은 수명의 signing identity를 사용한다.

## 4. macOS

출시 준비:

- 고정된 bundle identifier와 Apple Developer Team
- Developer ID Application certificate
- hardened runtime 및 필요한 entitlement 최소화
- DMG/앱 서명과 nested binary/sidecar 서명
- Apple notarization 제출과 ticket stapling
- `codesign --verify`, `spctl`, clean Mac 실행 검증

Entitlement는 기능 근거와 함께 review한다. 다운로드 실행, automation, keychain access, network listener 같은 권한을 포괄적으로 열지 않는다. Provider adapter/managed runtime이 앱 bundle 밖에 설치되면 해당 artifact의 signature/hash와 quarantine 처리 전략을 별도 검증한다.

## 5. Windows

출시 준비:

- organization identity의 code-signing certificate(가능하면 hardware-backed/managed)
- executable, DLL, installer, updater payload 서명
- RFC3161 timestamp
- signature chain/revocation 검증
- Windows 11 clean VM에서 install/upgrade/uninstall, standard-user/UAC 경로
- Defender/SmartScreen 관찰과 false-positive 대응 채널

Windows Credential Manager/DPAPI 데이터는 uninstall 정책에서 사용자 데이터와 분리한다. 시스템 package manager 또는 admin 설치 CLI를 silent update하지 않는다.

## 6. Harness App Update

Tauri 2 updater를 사용하되 다음 정책을 Core에서 강제한다.

1. HTTPS channel metadata fetch
2. metadata signature, expiry, channel, platform/architecture, current/minimum version 검증
3. disk/network/battery 정책 검사
4. staging download와 size/hash 검증
5. updater signature 검증
6. 사용자 policy에 따라 notify/download/install
7. 앱 종료 전 DB checkpoint/backup
8. installer handoff
9. 재시작 후 health/migration check
10. 실패 시 지원되는 플랫폼 rollback 또는 복구 안내

Downgrade는 서명됐더라도 기본 차단한다. 긴급 rollback은 별도 signed rollback authorization metadata와 DB compatibility를 요구한다.

설정 의미:

- `Automatic`: background download 후 안전한 시점/사용자 안내에 따라 설치
- `Download Automatically`: 다운로드까지만 자동, 설치는 사용자 확인
- `Notify Only`: 확인/표시만 수행

진행 중 Agent Run, unsaved composer, DB migration 중에는 restart하지 않는다.

## 7. Component Inventory

종류:

- `APP`
- `PROVIDER_ADAPTER`
- `CLI_AGENT`
- `LOCAL_RUNTIME`
- `LOCAL_MODEL`
- `PLUGIN`

공통 metadata:

```json
{
  "component_id": "...",
  "kind": "LOCAL_RUNTIME",
  "installed_version": "1.2.3",
  "latest_version": "1.3.0",
  "install_source": "APP_MANAGED",
  "update_policy": "AUTOMATIC",
  "status": "UPDATE_AVAILABLE",
  "compatibility": "COMPATIBLE",
  "last_checked": "RFC3339",
  "artifact_sha256": "...",
  "signature_identity": "..."
}
```

`latest_version`을 알 수 없으면 `null/unknown`이며 현재가 최신이라고 표시하지 않는다.

## 8. 설치 출처별 정책

| Source | Detect | Update 기본값 | Executor |
|---|---|---|---|
| `APP_MANAGED` | Harness inventory | Automatic 가능 | signed staged artifact + atomic swap |
| `VENDOR_NATIVE` | 공식 version/status | Notify Only | vendor 공식 updater, 명시 승인 |
| `HOMEBREW` | brew metadata | Notify Only | 안내 또는 승인된 brew command |
| `WINGET` | winget metadata | Notify Only | 안내 또는 승인된 winget command |
| `SYSTEM_PACKAGE` | OS/package DB | Manual | 시스템 관리자 경로 |
| `MANUAL` | path/version probe | Manual | 다운로드 안내, 임의 overwrite 금지 |
| `EXTERNAL` | enterprise/unknown | Manual/Blocked | 관리자 정책 |

Harness가 감지한 설치 출처를 자동으로 다른 방식으로 전환하지 않는다. 여러 executable 후보가 있으면 실제 resolved path를 표시하고 사용자가 선택하도록 한다.

## 9. Signed Component Metadata

권장 metadata는 TUF와 유사한 역할 분리를 갖는다(실제 TUF 채택 여부는 구현 ADR로 결정).

- Root: 장기 trust root와 threshold key, offline 보관
- Targets: artifact path/hash/size/version/platform/compatibility
- Snapshot: metadata version 일관성
- Timestamp: 짧은 expiry로 freeze 공격 방지

최소 target record:

```json
{
  "id": "runtime.llama.cpp",
  "version": "...",
  "platform": "macos-aarch64",
  "size": 123456,
  "sha256": "...",
  "url": "https://approved-host/...",
  "minimum_harness": "0.5.0",
  "maximum_harness_exclusive": null,
  "published_at": "RFC3339",
  "expires_at": "RFC3339",
  "signatures": []
}
```

URL host allowlist, redirect 제한, content length, decompression ratio, executable path를 검증한다. 다운로드 서버와 metadata 서버가 함께 침해되어도 pinned root 없이는 신뢰되지 않아야 한다.

## 10. Provider Adapter Update

Harness-managed adapter:

```text
download to unique staging
→ size/hash/signature/manifest/protocol/entrypoint validation
→ quarantine sandbox smoke + conformance subset
→ stop accepting new runs
→ wait/cancel existing runs per policy
→ atomic active-version pointer switch
→ detect/status smoke
→ success mark; previous N versions retain
```

실패하면 active pointer를 이전 verified version으로 되돌린다. 새 adapter가 DB migration을 직접 실행할 수 없으며 Core API를 통해 versioned state를 사용한다.

## 11. CLI Agent Update

- 실제 executable path와 설치 출처를 먼저 탐지한다.
- vendor-native updater의 availability/result만 해석하며 credential을 읽지 않는다.
- package manager 대상은 command와 영향 범위를 보여주고 사용자 승인을 받는다.
- 관리자 권한이 필요하면 silent escalation하지 않는다.
- enterprise-managed marker/policy가 있으면 update action을 비활성화한다.
- update 후 version/auth/status/protocol smoke를 실행한다.
- 외부 updater rollback은 Harness가 보장한다고 표시하지 않는다.

## 12. Local Runtime/Model

### Runtime

Harness data directory에 versioned install하고 `current` pointer만 atomic switch한다. OS/architecture/backend compatibility와 executable hash/signature를 검사한다. smoke inference가 실패하면 activate하지 않는다.

### Model

Model catalog는 앱과 독립적으로 signed update한다. 각 record에는 license, source, file size/hash, runtime/platform, RAM/VRAM 요구량, context/capability가 있어야 한다. license 수락이 필요한 모델은 background install 대상이 아니다.

다운로드 manager 요구:

- preflight disk budget(artifact + staging + rollback 여유)
- partial file과 resume metadata 분리
- range resume 시 ETag/Last-Modified/hash 일관성 확인
- pause/resume/cancel/retry
- metered network 기본 자동 다운로드 금지
- full SHA-256 후에만 final path로 이동
- pin된 Project version 자동 교체 금지

Model은 크기가 크므로 기본 `Notify Only`다.

## 13. Update All

다음 predicate를 모두 만족하는 component만 비대화식 자동 처리 후보가 된다.

```text
source == APP_MANAGED
AND policy == AUTOMATIC
AND signature/hash valid
AND compatibility == COMPATIBLE
AND no admin privilege
AND no unaccepted license
AND network/power/disk policy satisfied
AND component not pinned by active project
AND rollback capacity available
```

나머지는 grouped review 목록으로 보여주며 `Update All`이 임의로 우회하지 않는다.

## 14. Compatibility

Adapter record:

- minimum vendor CLI
- tested-through vendor CLI
- optional maximum-exclusive
- protocol range
- minimum Harness
- OS/architecture

결정:

- `< minimum`: block + update required
- `minimum..tested-through`: normal
- `> tested-through`이고 maximum 미만/없음: compatibility warning + read-only probe/conformance smoke
- `>= maximum-exclusive`: block 또는 explicit experimental override

Model/runtime도 동일하게 supported backend/quantization/context feature를 검사한다.

## 15. Rollback과 Recovery

Harness-managed adapter/runtime은 이전 verified version과 activation metadata를 보존한다. App update rollback은 OS installer/Tauri 지원 범위와 DB schema compatibility에 좌우된다.

Recovery 순서:

1. 새 process health marker 대기
2. timeout/crash/migration failure 감지
3. 사용자 데이터 원본과 pre-update backup 보존
4. compatible하면 이전 binary/component activate
5. 새 schema 때문에 불가능하면 recovery UI와 backup/export 제공
6. update history에 redacted error와 artifact identity 기록

Model file은 activation 실패만으로 삭제하지 않는다. 사용자가 확인할 수 있는 quarantined 상태로 둔 뒤 명시적 cleanup한다.

## 16. Signing Key 운영

- app, updater metadata, component/catalog 역할을 분리한다.
- root key는 offline/threshold 보관을 권장한다.
- CI는 최소 권한의 short-lived signing request만 수행한다.
- key access는 audit하고 개인 개발자 machine에 production key를 복사하지 않는다.
- rotation을 정상 release 전에 연습한다.
- compromised key revoke, affected version block, replacement trust chain 절차를 문서화한다.
- certificate/token 만료를 release calendar에서 사전 알림한다.

## 17. Staged Rollout

stable release는 내부 → beta → 5% → 25% → 100%로 확장할 수 있다. local-first 원칙상 개인 content telemetry를 요구하지 않는다. opt-in crash/health signal, update failure rate, support report를 사용한다. kill switch는 특정 app version을 원격 삭제하는 기능이 아니라 update 제공 중단과 compromised component block metadata로 한정한다.

## 18. Release Gate

- [ ] source/lockfile가 clean하고 immutable tag와 일치
- [ ] unit/integration/E2E/security/migration/provider contract 통과
- [ ] dependency vulnerability/license/secret scan, SBOM/provenance 생성
- [ ] macOS/Windows signed artifact 검증
- [ ] notarization/stapling 또는 Windows signature/timestamp 확인
- [ ] clean install, 이전 두 stable version upgrade, uninstall/repair
- [ ] DB backup/migration/recovery와 update rollback drill
- [ ] offline/proxy/metered/disk-full/interrupted download
- [ ] external-managed component가 자동 변경되지 않음
- [ ] release notes에 schema, security, preview Provider, known limitation 명시
- [ ] signed channel metadata publish 후 실제 client check
- [ ] 최종 artifact SHA-256, tag, commit, evidence archive 기록

Production signing credential과 실제 update endpoint가 준비되지 않은 build는 기능 demo가 가능해도 production release로 부르지 않는다.
