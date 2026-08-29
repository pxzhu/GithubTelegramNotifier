# SQLite Data Model과 Migration Plan

## 1. 원칙

- SQLite가 local metadata와 execution history의 source of truth다.
- foreign key를 켜고 모든 상태 변경을 transaction으로 수행한다.
- conversation/message/run/event는 append 중심이며 archive는 soft state다.
- secret, access/refresh token, vendor credential은 DB에 저장하지 않는다.
- 대용량 attachment/artifact/model/runtime는 content-addressed 파일 저장소에 두고 DB에는 hash/reference만 저장한다.
- Provider가 제거되거나 unavailable이어도 historical row는 보존한다.
- timestamp는 UTC RFC3339 또는 epoch millisecond 중 하나로 schema 전체에서 통일한다. UI에서만 local timezone으로 렌더링한다.

권장 pragma:

```sql
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA busy_timeout = 5000;
```

보안/내구성 요구에 따라 중요한 migration, backup, update transaction에서는 일시적으로 더 강한 sync 정책을 사용할 수 있다.

## 2. Entity 관계

```text
projects 1 ── N chat_groups
projects 1 ── N conversations 1 ── N messages
                         │              ├── N attachments
                         │              └── N artifacts
                         ├── N provider_sessions ── providers/models
                         └── N requests 1 ── N tasks 1 ── N agent_runs
                                                   │          ├── N agent_events
                                                   │          ├── N usage_events
                                                   │          └── N file_changes
                                                   └── task_dependencies

providers 1 ── N models
components 1 ── N component_versions / update_history
local_runtimes 1 ── N local_models
worker_nodes 1 ── N worker_capabilities / worker_leases (future)
```

## 3. 핵심 Table

### Project와 Conversation

#### `projects`

| Column | Type | 비고 |
|---|---|---|
| `id` | TEXT PK | UUID/ULID |
| `name` | TEXT | non-empty |
| `description` | TEXT | default empty |
| `workspace_path` | TEXT NULL | canonical path; 사용자 표시 path는 별도 가능 |
| `git_metadata_json` | TEXT NULL | branch/remote summary; credential 금지 |
| `instructions` | TEXT | project agent instructions |
| `agent_policy_json` | TEXT | versioned policy |
| `routing_policy_json` | TEXT | versioned policy |
| `budget_policy_json` | TEXT | versioned policy |
| `created_at`, `updated_at` | INTEGER | UTC epoch ms 예시 |
| `archived_at` | INTEGER NULL | soft archive |

Tag는 정규화된 `tags`, `project_tags`, `conversation_tags` table을 권장한다. 초기 단순 JSON에서 시작하더라도 검색/rename을 위해 migration 목표를 명시한다.

#### `chat_groups`

`id`, `project_id`, `name`, `sort_order`, `collapsed`, `created_at`, `updated_at`, `archived_at`. `UNIQUE(project_id, sort_order)`는 reorder transaction이 복잡하므로 sparse numeric rank 또는 lexicographic rank를 고려한다.

#### `conversations`

`id`, `project_id`, nullable `group_id`, `title`, `status`, `favorite`, `sort_order`, `provider_policy_json`, nullable `workspace_override`, `created_at`, `updated_at`, `archived_at`.

Group 삭제 시 conversation을 삭제하지 않고 `group_id`를 `NULL` 또는 명시적 Ungrouped group으로 이동한다.

#### `messages`

| Column | 의미 |
|---|---|
| `id`, `conversation_id` | identity/FK |
| `request_id` | 같은 사용자 요청/agent 결과 연결 |
| `role` | `USER`, `ASSISTANT`, `AGENT`, `SYSTEM` |
| `kind` | `TEXT`, `ERROR`, `RESULT`, `NOTICE` 등 |
| `content` | 최종 redacted/render source; raw HTML로 신뢰 금지 |
| `provider_id`, `model_id`, `agent_run_id` | nullable provenance |
| `sequence` | conversation 내 안정적 순서 |
| `created_at`, `edited_at`, `deleted_at` | lifecycle |
| `metadata_json` | schema-versioned optional data |

Stream delta마다 message row를 update하지 않는다. 메모리/temporary event batch 후 final message를 commit하며 crash 시 agent event journal에서 incomplete output을 복구할 수 있다.

### Attachment와 Artifact

`attachments`는 사용자 입력, `artifacts`는 agent/provider 산출물을 뜻한다. 공통 필드: `id`, owner reference, `storage_kind`, `relative_path`, `content_sha256`, `byte_size`, `mime_type`, `original_name`, `created_at`, `redaction_state`, `sensitivity`.

파일 저장소 규칙:

```text
data/
  blobs/sha256/ab/cd/<full-hash>
  exports/
  staging/<operation-id>/
  models/<catalog-id>/<version>/
```

DB에는 data root 기준 relative path만 저장하고 import 시 path traversal을 검사한다. blob 등록은 temp download → size/hash 검증 → atomic rename → DB insert 순서다.

### Provider와 Model

#### `providers`

`id`, `manifest_id`, `display_name`, `provider_type`, `enabled`, `status`, `adapter_version`, `detected_version`, `install_source`, `auth_state`, `capability_revision`, `last_probe_at`, `last_error_code`, `settings_json`.

`settings_json`에는 secret 값이 아니라 secure-store key ID만 허용한다.

#### `models`

`id`, `provider_id`, `provider_model_id`, `display_name`, `version`, `capabilities_json`, `context_length`, `availability`, `metadata_revision`, `first_seen_at`, `last_seen_at`, `retired_at`.

Provider runtime discovery에서 사라진 model을 바로 삭제하지 않고 retired/unavailable로 바꿔 historical reference를 유지한다.

#### `provider_sessions`

`id`, `conversation_id`, `provider_id`, nullable `model_id`, opaque `external_session_id`, `resume_capability`, `created_at`, `last_used_at`, `state`, `metadata_json`. external ID는 secret으로 간주될 가능성을 평가하고 로그에는 표시하지 않는다.

### Request, Task, Run, Event

#### `requests`

한 user message의 orchestration scope다. `id`, `conversation_id`, `user_message_id`, `routing_mode`, `status`, `policy_snapshot_json`, `router_decision_json`, `created_at`, `completed_at`.

정책은 실행 도중 Project 설정이 바뀌어도 audit 가능한 snapshot으로 저장한다.

#### `tasks`

`id`, `request_id`, nullable `parent_task_id`, `type`, `description`, `status`, `priority`, `capabilities_json`, nullable `preferred_provider_id`, nullable `assigned_provider_id`, nullable `assigned_model_id`, `risk`, `retry_policy_json`, `created_at`, `started_at`, `completed_at`, `version`.

`version`은 optimistic update/cancellation race 제어에 사용한다.

#### `task_dependencies`

복합 PK `(task_id, depends_on_task_id)`, dependency kind, created_at. 자기 참조를 금지하고 DAG cycle은 application transaction에서 검사한다. DB trigger만으로 전체 cycle 검증을 구현하지 않는다.

#### `agent_runs`

`id`, `task_id`, `attempt`, `provider_id`, `model_id`, nullable `provider_session_id`, nullable `parent_run_id`, `status`, `idempotency_key`, `started_at`, `completed_at`, `exit_code`, `error_code`, `error_summary`, `resume_token_ref`.

`UNIQUE(task_id, attempt)`, `UNIQUE(provider_id, idempotency_key)` 같은 제약을 적용하되 provider가 idempotency를 지원하지 않음을 별도 표시한다.

#### `agent_events`

`id`, `run_id`, `sequence`, `kind`, `occurred_at`, nullable `provider_time`, `payload_version`, `payload_json`, `redaction_state`, `created_at`. `UNIQUE(run_id, sequence)`와 `UNIQUE(event_id)`로 duplicate를 막는다.

Event payload는 검색/집계가 필요한 핵심 필드까지 JSON 안에 숨기지 않는다. usage, file change, approval은 전용 table에도 projection하며 event가 원본 audit link다.

#### `permission_decisions`

`id`, `request_id`, `task_id`, `run_id`, `operation_fingerprint`, `risk`, `tool`, `canonical_target`, `decision`, `scope`, `policy_revision`, `decided_at`, `expires_at`, `reason`. Prompt/content 전문은 저장하지 않는다.

#### `file_changes`

`id`, `run_id`, `path`, `change_kind`, `additions`, `deletions`, nullable `before_sha256`, nullable `after_sha256`, nullable `diff_artifact_id`, `created_at`. Path는 workspace-relative display와 canonical target을 구분할 수 있다.

### Usage

`usage_events` 권장 column:

```text
id, provider_id, model_id, project_id, conversation_id,
task_id, run_id, occurred_at, source, is_final,
input_tokens NULL, output_tokens NULL, cached_tokens NULL,
reasoning_tokens NULL, request_count NULL, turn_count NULL,
tool_calls NULL, duration_ms NULL, estimated_cost_minor NULL,
currency NULL, quota_units NULL, local_tokens NULL,
local_compute_ms NULL, raw_unit_json NULL, pricing_revision NULL
```

금액은 floating point가 아니라 minor unit 또는 decimal string을 사용한다. 동일 run의 incremental event와 final event 중복 집계를 막기 위해 aggregation view가 final 우선/delta semantics를 명시해야 한다.

### Component/Local AI

`components`: kind, component ID, installed/latest version, source, update policy, status, platform, last checked.

`component_versions`: component ID, version, install path reference, artifact hash, signature identity, installed/activated/retired timestamp, compatibility status.

`update_history`: check/plan/download/verify/install/activate/rollback event, result/error, actor/approval, artifact metadata.

`local_runtimes`: runtime ID/version/backend/platform/install status/smoke result.

`local_models`: catalog ID/version/family/quantization/path/hash/license acceptance/capabilities/pinned state/status.

Catalog metadata 자체와 설치된 artifact state를 분리한다.

## 4. FTS5

검색 대상은 conversation title, message content, final agent result, tag, 파일명이다. contentless/external-content FTS 중 하나를 선택하고 trigger 또는 명시적 indexing service로 동기화한다.

예시:

```sql
CREATE VIRTUAL TABLE search_fts USING fts5(
  entity_type UNINDEXED,
  entity_id UNINDEXED,
  project_id UNINDEXED,
  conversation_id UNINDEXED,
  title,
  body,
  tags,
  file_names,
  tokenize = 'unicode61 remove_diacritics 2'
);
```

고려사항:

- 한국어 형태소 분석을 기본 FTS5가 완전히 제공하지 않는다. 초기에는 Unicode substring/term 한계를 문서화하고, 필요 시 별도 tokenizer를 평가한다.
- sensitivity가 `SECRET`인 content는 indexing하지 않는 정책을 지원한다.
- archive filter는 FTS 결과를 source table과 join해 적용한다.
- DB migration/recovery 후 deterministic rebuild command를 제공한다.
- snippet은 UI에서 escape하고 control character/HTML을 실행하지 않는다.

## 5. Index

최소 index:

- conversations `(project_id, archived_at, updated_at DESC)`
- conversations `(group_id, sort_order)`
- messages `(conversation_id, sequence)`
- messages `(request_id)`
- tasks `(request_id, status, priority)`
- task_dependencies `(depends_on_task_id)`
- agent_runs `(task_id, attempt)`와 `(status, started_at)`
- agent_events `(run_id, sequence)`
- usage_events `(project_id, occurred_at)`, `(provider_id, occurred_at)`, `(run_id)`
- components `(kind, status)`
- local_models `(catalog_id, version)`

실제 query plan 측정 없이 index를 무한 추가하지 않는다.

## 6. Transaction invariant

- user message insert와 request 생성은 같은 transaction이다.
- task graph는 전체 node/dependency validation 후 한 transaction에 commit한다.
- terminal run event, run state, task projection, final usage는 가능한 한 한 transaction에 반영한다.
- Provider/session 삭제는 FK cascade로 conversation/message/task/run을 삭제하지 않는다.
- component activation은 verified filesystem atomic switch 성공 후 DB active pointer를 갱신한다.
- archive import는 staging DB/files 검증 후 merge하며 부분 import를 남기지 않는다.

## 7. Migration

### Version table

```sql
CREATE TABLE schema_migrations (
  version INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  checksum TEXT NOT NULL,
  applied_at INTEGER NOT NULL,
  app_version TEXT NOT NULL
);
```

원칙:

1. release된 migration은 수정하지 않고 새 migration으로 교정한다.
2. checksum mismatch는 startup을 중단하고 recovery UI를 연다.
3. migration 전 consistency check와 backup/snapshot을 만든다.
4. DDL/DML은 가능한 transaction으로 실행한다.
5. 큰 backfill은 resumable batch와 migration progress table을 사용한다.
6. 앱 downgrade가 새 schema를 열지 못하도록 최소 compatible schema를 기록한다.
7. migration 실패 시 backup 복원 또는 old DB 보존 + 새 DB repair 선택지를 제공한다.
8. destructive column/table drop은 최소 한 release deprecation window 후 수행한다.

### 권장 초기 migration 묶음

- `0001_core`: projects/groups/conversations/messages/settings
- `0002_search`: FTS와 rebuild metadata
- `0003_providers`: providers/models/sessions
- `0004_runtime`: requests/tasks/dependencies/runs/events/permissions/file changes
- `0005_usage`: normalized usage와 aggregation views
- `0006_components`: components/versions/update history/local runtime/model
- `0007_workers`: worker identity/capability/lease (Phase 13에서만)

초기 개발 중에도 한 거대 migration보다 domain별 경계를 유지해 failure를 찾기 쉽게 한다.

## 8. Backup, Export, Import

### 자동 Backup

- SQLite online backup API로 일관된 snapshot을 만든다.
- migration/update 전 backup은 보존 우선순위가 높다.
- retention은 사용자 설정과 disk budget을 따른다.
- backup metadata에 schema/app version, created time, DB SHA-256를 저장한다.
- restore는 원본을 덮기 전에 현재 DB를 별도 보존하고 integrity/foreign key check를 실행한다.

### Harness Archive

Manifest, DB logical export, blobs를 포함하는 versioned archive다. secret과 vendor credential은 포함하지 않는다. Manifest에는 file path, size, SHA-256, sensitivity, schema version을 둔다. Import는 압축 해제 전에 총 크기/파일 수 제한을 검사하고 zip-slip/symlink/hardlink를 거부한다.

ID 충돌 시 전체 import namespace mapping을 만들고 FK를 일관되게 재작성한다. Provider가 설치되지 않아도 imported historical provider/model label을 보존한다.

## 9. Retention과 삭제

- archive는 삭제가 아니다.
- conversation 영구 삭제 시 message/task/run/event/usage/blob reference 영향과 되돌릴 수 없음을 보여준다.
- shared blob은 reference count 또는 reachability sweep로 제거한다.
- secure-store credential 삭제는 DB row 삭제와 별도 성공 여부를 확인한다.
- audit/update history retention은 사용자가 명시적으로 줄일 수 있으나 외부 telemetry로 보내지 않는다.
- DB vacuum은 foreground 작업을 막지 않게 scheduling하고 충분한 disk를 확인한다.

## 10. 검증

- migration: 빈 DB, 각 과거 version, 중간 실패, disk full, checksum mismatch
- integrity: FK, uniqueness, DAG validation, terminal state race
- persistence: kill/restart, WAL recovery, concurrent reader/writer
- FTS: insert/edit/archive/delete/rebuild, Unicode/한국어, malicious markup
- scale: 10k conversation, 500k message, 수백만 event/usage row query p95
- export/import: round-trip, missing Provider, corrupt hash, zip bomb/path traversal
- privacy: secret canary가 DB/log/FTS/export 대상 밖에 있는지 확인
- backup: online writes 중 snapshot, restore, old app/schema incompatibility

DB schema는 application DTO를 그대로 복사하는 것이 아니라 durability, provenance, recovery를 위한 계약이다.
