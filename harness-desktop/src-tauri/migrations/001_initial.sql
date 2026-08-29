PRAGMA foreign_keys = ON;

CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL CHECK(length(trim(name)) > 0),
    description TEXT NOT NULL DEFAULT '',
    workspace_path TEXT,
    instructions TEXT NOT NULL DEFAULT '',
    routing_mode TEXT NOT NULL,
    status TEXT NOT NULL,
    tags_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(tags_json)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS chat_groups (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    name TEXT NOT NULL CHECK(length(trim(name)) > 0),
    sort_order INTEGER NOT NULL DEFAULT 0,
    collapsed INTEGER NOT NULL DEFAULT 0 CHECK(collapsed IN (0, 1)),
    archived INTEGER NOT NULL DEFAULT 0 CHECK(archived IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_chat_groups_project_order
    ON chat_groups(project_id, archived, sort_order);

CREATE TABLE IF NOT EXISTS conversations (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    group_id TEXT REFERENCES chat_groups(id) ON DELETE SET NULL,
    title TEXT NOT NULL CHECK(length(trim(title)) > 0),
    status TEXT NOT NULL,
    favorite INTEGER NOT NULL DEFAULT 0 CHECK(favorite IN (0, 1)),
    archived INTEGER NOT NULL DEFAULT 0 CHECK(archived IN (0, 1)),
    tags_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(tags_json)),
    provider_policy TEXT,
    workspace_path TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_conversations_project_updated
    ON conversations(project_id, archived, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_conversations_group
    ON conversations(group_id, updated_at DESC);

CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    provider_id TEXT,
    model_id TEXT,
    parent_message_id TEXT REFERENCES messages(id) ON DELETE SET NULL,
    metadata_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(metadata_json)),
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_messages_conversation_created
    ON messages(conversation_id, created_at);

CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
    content,
    content='messages',
    content_rowid='rowid',
    tokenize='unicode61 remove_diacritics 2'
);

CREATE TRIGGER IF NOT EXISTS messages_fts_insert AFTER INSERT ON messages BEGIN
    INSERT INTO messages_fts(rowid, content) VALUES (new.rowid, new.content);
END;
CREATE TRIGGER IF NOT EXISTS messages_fts_delete AFTER DELETE ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, content)
        VALUES ('delete', old.rowid, old.content);
END;
CREATE TRIGGER IF NOT EXISTS messages_fts_update AFTER UPDATE OF content ON messages BEGIN
    INSERT INTO messages_fts(messages_fts, rowid, content)
        VALUES ('delete', old.rowid, old.content);
    INSERT INTO messages_fts(rowid, content) VALUES (new.rowid, new.content);
END;

CREATE TABLE IF NOT EXISTS attachments (
    id TEXT PRIMARY KEY,
    message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    display_name TEXT NOT NULL,
    local_path TEXT NOT NULL,
    media_type TEXT,
    sha256 TEXT,
    size_bytes INTEGER,
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS artifacts (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    task_id TEXT,
    kind TEXT NOT NULL,
    display_name TEXT NOT NULL,
    local_path TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(metadata_json)),
    created_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS providers (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    provider_type TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1 CHECK(enabled IN (0, 1)),
    status TEXT NOT NULL,
    version TEXT,
    capabilities_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(capabilities_json)),
    status_detail TEXT,
    last_detected_at TEXT,
    settings_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(settings_json))
);

CREATE TABLE IF NOT EXISTS models (
    id TEXT PRIMARY KEY,
    provider_id TEXT NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
    display_name TEXT NOT NULL,
    version TEXT,
    capabilities_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(capabilities_json)),
    context_length INTEGER,
    is_local INTEGER NOT NULL DEFAULT 0 CHECK(is_local IN (0, 1)),
    metadata_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(metadata_json)),
    UNIQUE(provider_id, id)
);

CREATE TABLE IF NOT EXISTS provider_sessions (
    id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    provider_id TEXT NOT NULL,
    external_session_id TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(conversation_id, provider_id, external_session_id)
);

CREATE TABLE IF NOT EXISTS tasks (
    id TEXT PRIMARY KEY,
    parent_task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    role TEXT NOT NULL,
    description TEXT NOT NULL,
    status TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 0,
    capabilities_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(capabilities_json)),
    preferred_provider TEXT,
    assigned_provider TEXT,
    assigned_model TEXT,
    retry_count INTEGER NOT NULL DEFAULT 0,
    max_retries INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    started_at TEXT,
    completed_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_tasks_conversation_status
    ON tasks(conversation_id, status, priority DESC);

CREATE TABLE IF NOT EXISTS task_dependencies (
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    depends_on_task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    PRIMARY KEY(task_id, depends_on_task_id),
    CHECK(task_id <> depends_on_task_id)
);

CREATE TABLE IF NOT EXISTS agent_runs (
    id TEXT PRIMARY KEY,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    parent_run_id TEXT REFERENCES agent_runs(id) ON DELETE SET NULL,
    provider_id TEXT NOT NULL,
    model_id TEXT,
    status TEXT NOT NULL,
    provider_session_id TEXT,
    attempt INTEGER NOT NULL DEFAULT 1,
    error_code TEXT,
    error_message TEXT,
    started_at TEXT NOT NULL,
    completed_at TEXT
);

CREATE INDEX IF NOT EXISTS idx_agent_runs_task ON agent_runs(task_id, attempt);

CREATE TABLE IF NOT EXISTS agent_events (
    id TEXT PRIMARY KEY,
    run_id TEXT NOT NULL REFERENCES agent_runs(id) ON DELETE CASCADE,
    parent_run_id TEXT,
    task_id TEXT NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
    event_kind TEXT NOT NULL,
    sequence INTEGER NOT NULL,
    payload_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(payload_json)),
    created_at TEXT NOT NULL,
    UNIQUE(run_id, sequence)
);

CREATE TABLE IF NOT EXISTS usage_events (
    id TEXT PRIMARY KEY,
    provider_id TEXT NOT NULL,
    model_id TEXT,
    project_id TEXT REFERENCES projects(id) ON DELETE SET NULL,
    conversation_id TEXT REFERENCES conversations(id) ON DELETE SET NULL,
    task_id TEXT REFERENCES tasks(id) ON DELETE SET NULL,
    agent_run_id TEXT REFERENCES agent_runs(id) ON DELETE SET NULL,
    input_tokens INTEGER,
    output_tokens INTEGER,
    cached_tokens INTEGER,
    reasoning_tokens INTEGER,
    request_count INTEGER,
    turn_count INTEGER,
    tool_calls INTEGER,
    duration_ms INTEGER,
    estimated_cost REAL,
    quota_units REAL,
    local_tokens INTEGER,
    local_compute_ms INTEGER,
    tokens_per_second REAL,
    peak_memory_bytes INTEGER,
    raw_provider_usage_json TEXT CHECK(raw_provider_usage_json IS NULL OR json_valid(raw_provider_usage_json)),
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_usage_provider_created
    ON usage_events(provider_id, created_at);
CREATE INDEX IF NOT EXISTS idx_usage_project_created
    ON usage_events(project_id, created_at);

CREATE TABLE IF NOT EXISTS components (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    component_type TEXT NOT NULL,
    installed_version TEXT,
    latest_version TEXT,
    install_source TEXT NOT NULL,
    update_policy TEXT NOT NULL,
    status TEXT NOT NULL,
    enterprise_managed INTEGER NOT NULL DEFAULT 0 CHECK(enterprise_managed IN (0, 1)),
    last_checked_at TEXT,
    metadata_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(metadata_json))
);

CREATE TABLE IF NOT EXISTS component_versions (
    component_id TEXT NOT NULL REFERENCES components(id) ON DELETE CASCADE,
    version TEXT NOT NULL,
    installed_at TEXT NOT NULL,
    verified_sha256 TEXT,
    PRIMARY KEY(component_id, version)
);

CREATE TABLE IF NOT EXISTS update_history (
    id TEXT PRIMARY KEY,
    component_id TEXT NOT NULL REFERENCES components(id) ON DELETE CASCADE,
    from_version TEXT,
    to_version TEXT NOT NULL,
    status TEXT NOT NULL,
    detail TEXT,
    started_at TEXT NOT NULL,
    completed_at TEXT
);

CREATE TABLE IF NOT EXISTS local_runtimes (
    id TEXT PRIMARY KEY,
    runtime_kind TEXT NOT NULL,
    executable_path TEXT NOT NULL,
    version TEXT,
    managed INTEGER NOT NULL DEFAULT 0 CHECK(managed IN (0, 1)),
    status TEXT NOT NULL,
    detected_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS local_models (
    id TEXT PRIMARY KEY,
    runtime_id TEXT REFERENCES local_runtimes(id) ON DELETE SET NULL,
    display_name TEXT NOT NULL,
    version TEXT,
    local_path TEXT,
    size_bytes INTEGER,
    sha256 TEXT,
    license_id TEXT,
    license_accepted_at TEXT,
    pinned INTEGER NOT NULL DEFAULT 0 CHECK(pinned IN (0, 1)),
    status TEXT NOT NULL,
    metadata_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(metadata_json))
);

CREATE TABLE IF NOT EXISTS settings (
    scope TEXT NOT NULL,
    key TEXT NOT NULL,
    value_json TEXT NOT NULL CHECK(json_valid(value_json)),
    updated_at TEXT NOT NULL,
    PRIMARY KEY(scope, key)
);

CREATE TABLE IF NOT EXISTS workers (
    id TEXT PRIMARY KEY,
    display_name TEXT NOT NULL,
    endpoint TEXT,
    pairing_status TEXT NOT NULL,
    capabilities_json TEXT NOT NULL DEFAULT '[]' CHECK(json_valid(capabilities_json)),
    hardware_json TEXT NOT NULL DEFAULT '{}' CHECK(json_valid(hardware_json)),
    certificate_fingerprint TEXT,
    last_seen_at TEXT,
    enabled INTEGER NOT NULL DEFAULT 0 CHECK(enabled IN (0, 1))
);

CREATE TABLE IF NOT EXISTS routing_outcomes (
    id TEXT PRIMARY KEY,
    task_type TEXT NOT NULL,
    complexity TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    model_id TEXT,
    duration_ms INTEGER,
    usage_score REAL,
    succeeded INTEGER NOT NULL CHECK(succeeded IN (0, 1)),
    retried INTEGER NOT NULL DEFAULT 0 CHECK(retried IN (0, 1)),
    user_accepted INTEGER CHECK(user_accepted IS NULL OR user_accepted IN (0, 1)),
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_routing_outcome_provider
    ON routing_outcomes(provider_id, task_type, created_at);

PRAGMA user_version = 1;
