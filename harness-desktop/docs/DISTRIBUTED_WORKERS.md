# Distributed Worker Protocol (Phase 13)

## 1. 상태와 범위

이 문서는 future protocol 설계이며 현재 network listener를 활성화하는 근거가 아니다. Phase 13 전까지 Worker 기능은 build-time 또는 runtime feature flag로 꺼져 있어야 한다.

목표는 사용자가 명시적으로 pair한 PC에 bounded Task를 위임하는 것이다. 목표가 아닌 것:

- 공개 인터넷 P2P mesh
- 자동으로 발견한 LAN node 신뢰
- Main의 credential 복사
- 임의 원격 shell
- 여러 사용자가 공유하는 중앙 SaaS scheduler

## 2. 역할

- `Coordinator`: Desktop의 Scheduler. task lease를 발급하고 결과를 종합한다.
- `Worker`: 별도 daemon. 자신의 hardware/runtime/provider/repository capability를 광고하고 승인된 task를 실행한다.
- `Operator`: 두 기기를 pair하고 repository/tool/data boundary를 승인하는 사용자.
- `Relay`(선택, 후속): direct 연결이 불가능할 때 암호화된 frame만 전달하며 plaintext/task key를 보유하지 않는다.

Coordinator와 Worker는 서로를 완전히 신뢰하지 않는다. 각 task는 명시적인 capability lease로 제한한다.

## 3. Identity와 Pairing

각 node는 OS secure storage에 device identity private key를 생성한다. 공개키 fingerprint와 `node_id`를 저장하며 장기 private key는 export하지 않는다.

권장 pairing 흐름:

1. Operator가 양쪽 UI에서 `Pair a worker`를 시작한다.
2. discovery는 후보 주소/nonce만 제공하고 신뢰를 부여하지 않는다.
3. 한쪽이 QR 또는 one-time pairing secret을 표시한다.
4. Noise/TLS 기반 authenticated key exchange를 수행한다.
5. 양쪽 화면에 동일한 short authentication string과 node 이름/fingerprint를 표시한다.
6. Operator가 양쪽에서 확인한다.
7. 서로의 public identity, 허용 repository/capability, certificate/credential를 저장한다.
8. one-time secret을 폐기하고 pairing audit를 남긴다.

Pairing은 제한 시간, 시도 횟수, replay 방지를 가져야 한다. hostname/IP/MAC address는 identity가 아니다.

## 4. Transport

권장 baseline은 TLS 1.3 mutual authentication 위의 HTTP/2 또는 QUIC이다. 정확한 library 선택은 Phase 13 ADR에서 결정한다.

요구사항:

- mutual device authentication과 pinned identity
- protocol version negotiation
- stream별 flow control/backpressure
- request/message size와 concurrent stream limit
- keepalive와 bounded reconnect
- application-level sequence/idempotency
- 모든 control/data frame의 run/task/lease binding
- TLS 종료를 임의 reverse proxy에 맡기지 않음

LAN discovery는 mDNS/Bonjour 또는 OS discovery를 사용할 수 있지만 발견 payload에는 secret/capability 상세를 넣지 않는다. discovery 비활성/manual address mode도 제공한다.

## 5. Protocol Envelope

```json
{
  "protocol": "worker/1.0",
  "message_id": "uuid",
  "correlation_id": "uuid",
  "node_id": "uuid",
  "sent_at": "RFC3339",
  "kind": "TASK_OFFER",
  "payload_version": 1,
  "payload": {},
  "signature": "channel-bound-or-message-signature"
}
```

TLS만으로 충분한 framing integrity가 있어도 offline artifact/relay 확장을 위해 message identity와 replay cache를 유지한다. clock skew에 의존하지 않고 monotonic sequence/nonce를 함께 사용한다.

## 6. Node Capability

```json
{
  "node_id": "uuid",
  "display_name": "Office PC",
  "os": "windows",
  "architecture": "x86_64",
  "agent_version": "0.13.0",
  "protocol_range": { "min": "1.0", "max": "1.0" },
  "hardware": {
    "cpu": { "logical_cores": 16, "features": [] },
    "ram_bytes": 34359738368,
    "gpu": [{ "vendor": "nvidia", "model": "redacted-or-display", "vram_bytes": 8589934592 }]
  },
  "providers": [
    {
      "id": "local.llama.cpp",
      "capabilities": ["classification", "summarization", "local", "offline"],
      "models": ["catalog:model@version"],
      "auth_scope": "worker-local"
    }
  ],
  "repositories": [
    { "repository_id": "opaque-shared-id", "access": ["read"], "revision": "optional" }
  ],
  "limits": {
    "max_parallel_runs": 2,
    "available_disk_bytes": 50000000000,
    "max_artifact_bytes": 1000000000
  },
  "availability": "AVAILABLE",
  "revision": 31,
  "expires_at": "RFC3339"
}
```

Coordinator는 hostname/path/provider account를 필요 이상 저장하지 않는다. Capability는 short-lived snapshot이며 scheduling 직전에 재확인한다.

## 7. Repository Mapping

로컬 path를 네트워크로 그대로 공유하지 않는다. Pairing/Project 설정에서 `repository_id`를 생성하고 각 node의 canonical local root에 매핑한다.

모드:

1. `PREEXISTING_CLONE`: Worker에 이미 clone이 있고 revision을 fetch/checkout하는 것은 별도 permission에 따른다.
2. `SNAPSHOT_TRANSFER`: Coordinator가 필요한 파일의 manifest/blob snapshot을 보낸다. secret/ignored file 정책을 적용한다.
3. `ARTIFACT_ONLY`: source 접근 없이 input artifact만 처리한다.

초기 구현은 안전성이 높은 `ARTIFACT_ONLY`와 read-only `PREEXISTING_CLONE`부터 시작한다. remote write를 자동으로 Main workspace에 적용하지 않는다. patch artifact를 받고 Main에서 diff/approval/test 후 apply한다.

## 8. Task Lease

Coordinator가 발급하는 lease:

```json
{
  "lease_id": "uuid",
  "task_id": "uuid",
  "run_id": "uuid",
  "worker_node_id": "uuid",
  "issued_at": "RFC3339",
  "not_before": "RFC3339",
  "expires_at": "RFC3339",
  "capabilities": ["summarization", "file.read"],
  "repository": {
    "id": "opaque-repo-id",
    "mode": "ARTIFACT_ONLY",
    "revision": "sha256-or-git-oid"
  },
  "tools": [
    { "name": "file.read", "risk": "SAFE", "path_globs": ["docs/**"] }
  ],
  "network": { "mode": "provider-only", "hosts": [] },
  "limits": {
    "wall_time_ms": 600000,
    "cpu_time_ms": null,
    "memory_bytes": 8589934592,
    "output_bytes": 10485760,
    "artifact_bytes": 104857600
  },
  "input_manifest_sha256": "...",
  "idempotency_key": "uuid",
  "approval_fingerprint": "sha256:..."
}
```

Worker는 local policy와 lease의 교집합만 허용한다. capability가 부족하거나 local operator 승인이 필요하면 실행 전 거부/대기 응답을 보낸다. Coordinator가 부여한 권한은 Worker local policy보다 강하지 않다.

## 9. Task Lifecycle

```text
Coordinator                       Worker
    │──── CAPABILITY_QUERY ─────────→│
    │←── CAPABILITY_SNAPSHOT ────────│
    │──── TASK_OFFER ───────────────→│
    │←── TASK_ACCEPT / REJECT ───────│
    │──── INPUT_MANIFEST/CHUNKS ────→│
    │←── INPUT_VERIFIED ─────────────│
    │──── TASK_START ───────────────→│
    │←── RUN_EVENT(seq N) ───────────│
    │──── EVENT_ACK ────────────────→│
    │──── CANCEL (optional) ────────→│
    │←── RESULT_MANIFEST/TERMINAL ───│
    │──── RESULT_ACK/LEASE_CLOSE ───→│
```

`TASK_ACCEPT`만으로 실행하지 않고 input 검증 후 explicit `TASK_START`를 사용하면 잘못된/stale transfer의 실행을 막을 수 있다. task lease가 만료되면 새 외부 action/write를 시작하지 않고 안전하게 중단한다.

## 10. Event와 Backpressure

Provider normalized Agent Event를 재사용하되 `worker_node_id`, `lease_id`, worker sequence를 추가한다.

- Worker는 event를 local journal에 먼저 append한다.
- Coordinator ACK 이전 event를 bounded buffer로 보존한다.
- reconnect 시 마지막 ACK sequence부터 재전송한다.
- 동일 event ID/sequence는 Coordinator가 deduplicate한다.
- buffer limit 초과 시 verbose progress/delta를 coalesce할 수 있지만 tool/usage/file/terminal event는 버리지 않는다.
- raw chain-of-thought는 전송하지 않는다.

## 11. Artifact Transfer

Manifest:

```json
{
  "transfer_id": "uuid",
  "artifacts": [
    {
      "artifact_id": "uuid",
      "relative_path": "results/patch.diff",
      "media_type": "text/x-diff",
      "size": 12345,
      "sha256": "...",
      "chunks": 1,
      "sensitivity": "PROJECT"
    }
  ],
  "total_size": 12345,
  "manifest_sha256": "..."
}
```

수신자는 전송 전 총 크기/개수/disk budget/data label을 검사한다. path traversal, absolute path, device name, ADS, symlink/hardlink를 거부한다. chunk별 hash와 final hash를 검사하고 staging에서만 조립한 뒤 atomic register한다. dedup blob을 사용해도 다른 Project authorization을 우회하지 않는다.

## 12. Cancellation과 Failure

Cancellation 단계:

1. Coordinator가 cancel reason과 deadline을 전송한다.
2. Worker가 새 tool/subagent 시작을 막는다.
3. Provider/subprocess cooperative cancel.
4. timeout 후 descendant 강제 종료.
5. `CANCELLED` terminal과 partial artifact manifest 반환.
6. Coordinator가 late event를 terminal 이후 UI result로 적용하지 않고 audit로만 보존한다.

Disconnect 처리:

- read-only/idempotent task: lease 내에서 제한적으로 계속 가능하도록 Project 정책 선택
- modify/external task: 연결 상실 시 기본 pause/cancel
- lease expiry: 강제 중단
- Coordinator 재시작: journal sequence로 reconnect
- Worker 재시작: run을 `INTERRUPTED`로 보고; 지원 Provider만 resume

동일 task를 다른 Worker에 재할당할 때 idempotency와 기존 lease revoke를 확인한다. split-brain 상태에서 두 write result를 자동 적용하지 않는다.

## 13. Scheduling

Hard constraint:

- protocol/app compatibility
- paired/trusted/not revoked
- required provider/model/capability
- repository/data label/tool permission
- online/lease duration/disk/memory
- Project의 allowed nodes

Soft score:

```text
quality_fit - queue_delay - transfer_cost - latency
- quota_pressure - historical_failure - energy_policy_penalty
```

Worker 자기 보고만으로 RAM/VRAM 여유나 모델 정상 상태를 보장하지 않는다. probe freshness와 최근 health를 고려한다. laptop battery/metered network 정책을 존중한다.

## 14. Security

- Main의 Microsoft/vendor token을 Worker에 복사하지 않는다.
- Worker cloud Provider는 Worker가 직접 로그인한 session을 사용하며 capability로만 알린다.
- prompt/repository content는 untrusted input이고 Worker policy를 바꿀 수 없다.
- LAN 주소와 discovery payload를 신뢰 근거로 쓰지 않는다.
- paired key revoke는 즉시 새 lease를 막고 진행 중 lease 취소를 시도한다.
- long-lived node key와 short-lived session/lease key를 분리한다.
- audit에는 content 대신 identity fingerprint, capability, operation, result를 기록한다.
- OS firewall 안내를 제공하고 listener는 pair/setup 중 명시적으로 켠다.
- relay를 추가해도 end-to-end encryption과 peer identity pinning을 유지한다.

## 15. Versioning

- transport protocol major/minor negotiation
- capability vocabulary revision
- Task/Event/Artifact payload schema version
- minimum/maximum compatible daemon/app version

minor version은 optional field를 추가할 수 있다. required semantic 변화는 major version이다. Coordinator는 이해하지 못하는 capability를 routing에 사용하지 않으며 Worker는 이해하지 못하는 permission/limit이 있는 lease를 거부한다.

## 16. Test Plan

### 단일 process simulation

- handshake/version, lease acceptance/rejection
- event duplicate/out-of-order/reconnect
- cancel race와 lease expiry
- artifact corrupt/oversize/path traversal
- capability revision race

### 2-node 실제 환경

- macOS Coordinator ↔ Windows Worker와 역방향
- Wi-Fi disconnect/reconnect, IP 변경, sleep/wake
- firewall/proxy, clock skew
- Worker crash/restart와 orphan process
- large artifact resume, low disk
- repository revision mismatch와 conflicting patch
- pairing MITM/SAS mismatch/replay/revoke
- old/new version skew

### Exit Criteria

1. 양쪽 사용자가 확인하지 않은 node는 task를 받을 수 없다.
2. lease 밖 path/tool/network access가 거부된다.
3. cancel/expiry 후 descendant process가 남지 않는다.
4. artifact hash/path 검증 실패가 workspace를 변경하지 않는다.
5. 연결 손실/재시도에도 terminal state가 하나이고 event가 중복 집계되지 않는다.
6. Worker 손실이 Main DB/Conversation을 손상하지 않는다.
7. remote result는 provenance와 diff/review 없이 자동 적용되지 않는다.

이 기준을 통과하기 전에는 Distributed Worker를 일반 사용자 설정에 노출하지 않는다.
