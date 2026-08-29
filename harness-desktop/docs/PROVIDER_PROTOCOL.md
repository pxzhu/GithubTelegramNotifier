# Provider Protocol 및 Manifest

## 1. 목적

Provider 계층은 Claude, M365, Codex, local runtime, custom CLI의 인증·세션·stream 차이를 Core에서 분리한다. 이 문서는 built-in Rust adapter와 out-of-process adapter가 공유하는 의미 계약을 정의한다.

규범 용어 `MUST`, `SHOULD`, `MAY`는 각각 필수, 권장, 선택을 의미한다. 초기 protocol version은 예시로 `1.0`을 사용하며 배포 전에 registry와 fixture를 고정해야 한다.

## 2. Core SPI

개념 interface:

```text
AgentProvider
  detect(context) -> Detection
  status(context) -> ProviderStatus
  version() -> ComponentVersion
  capabilities() -> CapabilitySet
  models() -> [ModelDescriptor]
  authenticate(request) -> AuthOutcome
  execute(ExecutionRequest) -> Stream<AgentEvent>
  cancel(run_id) -> CancelOutcome
  resume(ResumeRequest) -> Stream<AgentEvent>
  usage(scope) -> UsageSnapshot | Unsupported
  check_update() -> UpdateAvailability
  install(plan, approval) -> InstallOutcome
  update(plan, approval) -> UpdateOutcome
```

모든 method는 typed error를 반환하고 process panic/exception을 protocol error로 격리한다. `install`/`update`는 manifest에 전략만 선언했다고 자동 허용되지 않으며 Component Manager 정책과 별도 사용자 승인을 통과해야 한다.

## 3. Transport

외부 adapter는 Harness가 child process로 시작하고 stdin/stdout의 newline-delimited JSON-RPC 2.0을 사용한다.

- UTF-8, 한 줄에 JSON object 하나다.
- stdout에는 protocol frame만 쓴다. 로그는 stderr로 보낸다.
- frame 기본 상한은 1 MiB이며 handshake에서 낮출 수 있다.
- 큰 artifact/binary는 허용된 staging directory의 reference와 SHA-256으로 전달한다.
- 각 request에는 `request_id`, 각 실행에는 Harness가 발급한 `run_id`가 있다.
- event에는 run별 단조 증가 `sequence`가 있다.
- Host는 알 수 없는 notification을 무시할 수 있지만 required response 누락은 protocol error로 처리한다.
- heartbeat 미응답, EOF, malformed JSON은 adapter crash로 간주하고 해당 run만 종료한다.

## 4. Handshake

Host가 먼저 `harness.initialize`를 호출한다.

```json
{
  "jsonrpc": "2.0",
  "id": "init-1",
  "method": "harness.initialize",
  "params": {
    "protocol": { "min": "1.0", "max": "1.0" },
    "host_version": "0.1.0",
    "platform": { "os": "macos", "arch": "aarch64" },
    "locale": "ko-KR",
    "max_frame_bytes": 1048576,
    "staging_root": "/canonical/ephemeral/path",
    "features": ["event-ack", "resume"]
  }
}
```

Adapter는 선택한 protocol, 실제 identity, capability snapshot을 반환한다.

```json
{
  "jsonrpc": "2.0",
  "id": "init-1",
  "result": {
    "protocol": "1.0",
    "adapter": { "id": "com.example.agent", "version": "1.2.0" },
    "provider": { "id": "example", "display_name": "Example Agent" },
    "capability_revision": "sha256:...",
    "features": ["event-ack"]
  }
}
```

Manifest ID와 handshake ID가 다르면 Host는 adapter를 거부한다. compatible protocol 교집합이 없으면 실행하지 않는다.

## 5. JSON-RPC method

| Method | 방향 | 의미 |
|---|---|---|
| `harness.initialize` | Host → Adapter | protocol 협상과 identity 검증 |
| `provider.initialize` | Host → Adapter | protocol range, host/platform, frame limit 협상 |
| `provider.getStatus` | Host → Adapter | auth/connectivity/degraded 상태 |
| `provider.getCapabilities` | Host → Adapter | 현재 capability snapshot과 revision 조회 |
| `provider.getModels` | Host → Adapter | 현재 사용 가능한 model snapshot |
| `provider.authenticate` | Host → Adapter | 공식 auth flow 시작; token 반환 금지 |
| `provider.execute` | Host → Adapter | 새 run 시작, 즉시 accepted 응답 |
| `provider.resume` | Host → Adapter | vendor session에서 실행 재개 |
| `provider.cancel` | Host → Adapter | cooperative cancellation 요청 |
| `provider.getUsage` | Host → Adapter | 지원하는 범위의 usage 조회 |
| `provider.shutdown` | Host → Adapter | 새 run 중지 후 정상 종료 |
| `agent.event` | Adapter → Host | normalized streaming event notification |
| `agent.eventAck` | Host → Adapter | 선택적 backpressure/영속화 확인 |
| `provider.statusChanged` | Adapter → Host | auth/update/connectivity 변화 |

`execute`가 accepted를 반환한 뒤 모든 run은 정확히 하나의 terminal event(`COMPLETED`, `FAILED`, `CANCELLED`, `INTERRUPTED`)로 끝나야 한다. process crash처럼 adapter가 terminal event를 보낼 수 없으면 Host가 synthetic `INTERRUPTED`를 기록한다.

## 6. Execution Request

```json
{
  "run_id": "uuid",
  "task_id": "uuid",
  "conversation": {
    "provider_session_id": null,
    "messages": [
      { "role": "user", "parts": [{ "type": "text", "text": "..." }] }
    ]
  },
  "model": "provider-model-id",
  "capabilities_required": ["coding", "file.read"],
  "workspace": {
    "root": "/canonical/project/root",
    "read_paths": ["/canonical/project/root"],
    "write_paths": [],
    "symlink_policy": "deny-escape"
  },
  "tools": [
    { "name": "file.read", "risk": "SAFE", "approval": "AUTO" }
  ],
  "limits": {
    "deadline": "RFC3339",
    "max_output_bytes": 10485760,
    "max_tool_calls": 100,
    "budget_units": null
  },
  "policy": {
    "network": "provider-only",
    "data_labels_allowed": ["PROJECT"],
    "raw_reasoning": "discard"
  },
  "idempotency_key": "uuid"
}
```

Adapter는 path/tool/network 권한을 넓힐 수 없다. 지원하지 않는 제한은 실행 전 `CAPABILITY_UNAVAILABLE`로 거부하며 조용히 무시하지 않는다.

## 7. Normalized Agent Event

공통 필드:

```json
{
  "run_id": "uuid",
  "sequence": 7,
  "event_id": "uuid",
  "kind": "TOOL_STARTED",
  "occurred_at": "RFC3339",
  "payload_version": 1,
  "payload": {},
  "provider_metadata": {
    "session_id": "opaque-or-null",
    "model": "provider-model-id"
  }
}
```

필수 event kind:

- Lifecycle: `STARTED`, `COMPLETED`, `FAILED`, `CANCEL_REQUESTED`, `CANCELLED`, `INTERRUPTED`
- Output: `TEXT_DELTA`, `RESULT`, `ARTIFACT_CREATED`
- Tool: `TOOL_APPROVAL_REQUIRED`, `TOOL_STARTED`, `TOOL_COMPLETED`, `TOOL_FAILED`
- Hierarchy: `SUBAGENT_STARTED`, `SUBAGENT_COMPLETED`
- Resource: `USAGE`, `FILE_CHANGED`, `PROGRESS`
- Session: `SESSION_CREATED`, `SESSION_RESUMED`

`TEXT_DELTA`는 transient하게 batch할 수 있지만 최종 message와 terminal event는 transaction으로 영속화한다. `FILE_CHANGED`에는 content 전체가 아니라 path, change kind, additions/deletions, optional diff artifact reference를 둔다.

### Usage payload

```json
{
  "input_tokens": 1200,
  "output_tokens": 350,
  "cached_tokens": null,
  "reasoning_tokens": null,
  "request_count": 1,
  "turn_count": null,
  "tool_calls": 4,
  "duration_ms": 8400,
  "estimated_cost": null,
  "currency": null,
  "quota_units": null,
  "local_tokens": null,
  "local_compute_ms": null,
  "source": "provider-event",
  "is_final": true
}
```

누락값은 `null`이어야 한다. adapter가 만든 추정치는 `source: estimated`와 계산 버전을 명시하지 않으면 허용하지 않는다.

## 8. Error 계약

JSON-RPC transport error와 provider execution error를 구분한다.

```json
{
  "code": "AUTH_REQUIRED",
  "message": "Sign in with the vendor CLI",
  "retryable": false,
  "retry_after_ms": null,
  "user_action": "OPEN_PROVIDER_SETTINGS",
  "details": { "redacted": true }
}
```

표준 code: `NOT_INSTALLED`, `AUTH_REQUIRED`, `AUTH_EXPIRED`, `PERMISSION_DENIED`, `MODEL_UNAVAILABLE`, `CAPABILITY_UNAVAILABLE`, `RATE_LIMITED`, `TIMEOUT`, `PROTOCOL_ERROR`, `EXECUTION_FAILED`, `CANCELLED`, `POLICY_BLOCKED`, `INTEGRITY_FAILURE`, `UPDATE_REQUIRED`, `INCOMPATIBLE_VERSION`.

message/details에는 token, prompt 전문, enterprise content, 로컬 파일 본문을 넣지 않는다.

## 9. Capability vocabulary

현재 manifest v1 registry는 `coding`, `reasoning`, `company_search`, `file_access`, `shell`, `git`, `tool_calling`, `vision`, `long_context`, `local`, `offline`, `cheap`, `fast`, `parallel_agents`, `session_resume`다. Schema에 없는 capability를 manifest metadata에 임의 문자열로 추가하면 stable routing에서 사용하지 않는다. `company.chat`, 세부 file read/write, summarization/classification/review처럼 더 세밀한 vocabulary는 schema v2 ADR에서 compatibility mapping과 함께 추가한다.

Capability는 권한이 아니다. 예를 들어 `shell` capability가 있어도 request의 tool grant와 사용자 정책이 없으면 실행할 수 없다.

## 10. Provider Manifest

저장소의 normative schema는 `providers/manifest.schema.json`이고 각 built-in manifest는 `providers/*.json`에 둔다. 외부 adapter 패키지는 `provider.manifest.json`을 포함하고 catalog가 별도 SHA-256/signature를 제공한다.

```json
{
  "$schema": "./manifest.schema.json",
  "schema_version": 1,
  "id": "anthropic.claude-code",
  "name": "Claude Code",
  "provider_type": "CLI_PROVIDER",
  "adapter_version": "0.1.0",
  "binary_candidates": ["claude", "claude.exe"],
  "supported_os": ["macos", "windows", "linux"],
  "capabilities": ["coding", "reasoning", "file_access", "shell", "git", "tool_calling", "long_context", "parallel_agents", "session_resume"],
  "auth_strategy": "INHERIT_CLI_SESSION",
  "install_strategy": "VENDOR_NATIVE",
  "update_strategy": "VENDOR_OR_PACKAGE_MANAGER",
  "usage_strategy": "EVENT_STREAM",
  "minimum_version": null,
  "tested_through": null,
  "models": [
    {
      "id": "dynamic",
      "display_name": "Models reported by Claude Code",
      "discovery": "DYNAMIC",
      "capabilities": ["coding", "reasoning"]
    }
  ],
  "metadata": {
    "credential_access": "forbidden",
    "event_format": "stream-json"
  }
}
```

### JSON Schema 핵심 제약

현재 v1 schema의 핵심 제약은 다음과 같다. 아래 요약보다 저장소의 schema 파일이 우선한다.

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "additionalProperties": false,
  "required": [
    "schema_version", "id", "name", "provider_type", "adapter_version",
    "supported_os", "capabilities", "auth_strategy",
    "install_strategy", "update_strategy", "usage_strategy"
  ],
  "properties": {
    "schema_version": { "const": 1 },
    "id": { "type": "string", "pattern": "^[a-z][a-z0-9.-]{2,63}$" },
    "name": { "type": "string", "minLength": 1, "maxLength": 80 },
    "provider_type": {
      "enum": ["CLI_PROVIDER", "HTTP_PROVIDER", "LOCAL_RUNTIME_PROVIDER", "CUSTOM_PROVIDER"]
    },
    "adapter_version": { "type": "string", "pattern": "^[0-9]+\\.[0-9]+\\.[0-9]+" },
    "supported_os": { "type": "array", "items": { "enum": ["macos", "windows", "linux"] } },
    "auth_strategy": { "enum": ["INHERIT_CLI_SESSION", "OAUTH_PKCE_OS_STORE", "NONE", "CUSTOM"] },
    "install_strategy": { "enum": ["VENDOR_NATIVE", "APP_MANAGED", "HOMEBREW", "WINGET", "EXTERNAL", "CUSTOM"] },
    "update_strategy": { "enum": ["VENDOR_OR_PACKAGE_MANAGER", "SIGNED_APP_MANAGED", "NOTIFY_ONLY", "CUSTOM"] },
    "usage_strategy": { "enum": ["EVENT_STREAM", "REQUEST_COUNT", "LOCAL_RUNTIME_METRICS", "UNAVAILABLE", "CUSTOM"] }
  }
}
```

검증기는 다음 semantic rule도 적용한다.

- `CLI_PROVIDER`에는 비어 있지 않은 `binary_candidates`가 필요하다.
- `HTTP_PROVIDER`에서 endpoint secret을 inline manifest에 넣을 수 없다.
- `INHERIT_CLI_SESSION` manifest에는 `metadata.credential_access: forbidden`을 요구하는 semantic validation을 추가해야 한다.
- binary candidate는 basename만 허용하고 path, option, shell fragment를 거부한다.
- 외부 adapter executable/hash/platform entrypoint는 manifest v2 또는 별도 signed package metadata가 담당한다. v1 manifest만으로 binary trust를 완료했다고 간주하지 않는다.
- unknown capability는 experimental namespace가 아니면 stable routing에 사용하지 않는다.
- `tested_through` 초과 버전은 경고/compatibility probe 후 실행한다.

## 11. Model Descriptor

```json
{
  "id": "opaque-provider-model-id",
  "display_name": "Model Name",
  "version": null,
  "capabilities": ["coding", "reasoning"],
  "context_length": null,
  "input_modalities": ["text"],
  "output_modalities": ["text"],
  "availability": "AVAILABLE",
  "cost_tier": "UNKNOWN",
  "latency_tier": "MEDIUM",
  "local": false,
  "metadata_revision": "provider-runtime"
}
```

모델 ID/display name/capability가 runtime에서 제공되지 않으면 추측하지 않는다. Catalog override는 출처와 유효 기간을 갖는다.

## 12. Auth 원칙

- CLI adapter는 공식 auth status command 또는 실제 command의 typed 결과만 사용한다.
- vendor credential/config 파일을 탐색하거나 parsing하지 않는다.
- OAuth adapter는 token을 protocol frame에 보내지 않는다. Adapter/Core가 OS secure store reference를 사용한다.
- system browser를 사용하며 embedded WebView password login을 기본값으로 하지 않는다.
- logout은 해당 Provider의 Harness-owned credential만 제거하며 vendor CLI session을 임의 삭제하지 않는다.

2026-08-29 기준 공식 OpenAI documentation은 `codex login status`가 로그인 상태에서 exit 0을 반환하고, `codex exec --json`이 newline-delimited JSON event를 출력하며, `codex exec resume [SESSION_ID]`가 non-interactive session을 이어가는 것으로 설명한다. Harness는 이 공개 command surface만 사용하고 release마다 version matrix를 다시 검증한다. [Codex CLI command reference](https://developers.openai.com/codex/cli/reference)

## 13. Versioning

- additive optional field/event: minor protocol 호환
- required field 의미 변경, method 제거: major protocol
- manifest schema와 transport protocol version은 독립적이다.
- adapter는 자신이 이해하지 못하는 major version을 거부한다.
- Host는 raw vendor payload 변화가 normalized contract에 새 field를 요구하지 않으면 protocol을 올리지 않는다.
- compatibility matrix에는 최소, tested-through, optional maximum-exclusive를 저장한다.

## 14. Conformance Suite

모든 adapter는 다음 fixture를 통과해야 한다.

1. handshake identity/protocol success와 mismatch 거부
2. not-installed/login-required/connected/degraded 상태
3. text streaming과 정확히 하나의 terminal event
4. tool event, approval wait, denial
5. cancel race: start 전, 실행 중, terminal 직후
6. malformed/truncated/oversized frame과 stdout 오염
7. crash/timeout/heartbeat loss 후 run isolation
8. duplicate/out-of-order event 처리
9. session create/resume unavailable/expired
10. usage null semantics와 final reconciliation
11. path traversal/symlink escape/artifact hash mismatch
12. secret canary가 event/error/stderr에 노출되지 않음
13. unsupported capability가 실행 전 명확히 거부됨
14. version 경고와 incompatible block

실제 Provider 출시에는 이 suite 외에 공식 CLI/API와의 외부 version matrix가 필요하다.
