use crate::domain::*;
use crate::error::{HarnessError, HarnessResult};
use chrono::{DateTime, Utc};
use rusqlite::types::Type;
use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;
use std::time::Duration;

const SCHEMA_VERSION: i64 = 1;
const INITIAL_MIGRATION: &str = include_str!("../migrations/001_initial.sql");

pub struct Database {
    connection: Connection,
}

impl Database {
    pub fn open(path: impl AsRef<Path>) -> HarnessResult<Self> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let connection = Connection::open(path)?;
        Self::initialize(connection, false)
    }

    pub fn in_memory() -> HarnessResult<Self> {
        Self::initialize(Connection::open_in_memory()?, true)
    }

    fn initialize(mut connection: Connection, in_memory: bool) -> HarnessResult<Self> {
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "trusted_schema", "OFF")?;
        if !in_memory {
            connection.pragma_update(None, "journal_mode", "WAL")?;
            connection.pragma_update(None, "synchronous", "NORMAL")?;
        }

        let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version > SCHEMA_VERSION {
            return Err(HarnessError::Unavailable(format!(
                "database schema {version} is newer than supported schema {SCHEMA_VERSION}"
            )));
        }
        if version == 0 {
            let transaction = connection.transaction()?;
            transaction.execute_batch(INITIAL_MIGRATION)?;
            transaction.commit()?;
        }
        let migrated_version: i64 =
            connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if migrated_version != SCHEMA_VERSION {
            return Err(HarnessError::Unavailable(format!(
                "database migration stopped at schema {migrated_version}"
            )));
        }
        Ok(Self { connection })
    }

    pub fn schema_version(&self) -> HarnessResult<i64> {
        Ok(self
            .connection
            .pragma_query_value(None, "user_version", |row| row.get(0))?)
    }

    pub fn integrity_check(&self) -> HarnessResult<bool> {
        let result: String = self
            .connection
            .query_row("PRAGMA quick_check", [], |row| row.get(0))?;
        Ok(result == "ok")
    }

    pub fn create_project(&mut self, input: NewProject) -> HarnessResult<Project> {
        let name = required_text("project name", &input.name, 200)?;
        let now = Utc::now();
        let project = Project {
            id: new_id(),
            name,
            description: input.description.trim().to_owned(),
            workspace_path: nonempty_optional(input.workspace_path),
            instructions: input.instructions.trim().to_owned(),
            routing_mode: input.routing_mode,
            status: ProjectStatus::Active,
            tags: normalize_tags(input.tags)?,
            created_at: now,
            updated_at: now,
        };
        self.connection.execute(
            "INSERT INTO projects
             (id, name, description, workspace_path, instructions, routing_mode, status,
              tags_json, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                project.id,
                project.name,
                project.description,
                project.workspace_path,
                project.instructions,
                enum_key(&project.routing_mode)?,
                enum_key(&project.status)?,
                serde_json::to_string(&project.tags)?,
                timestamp(project.created_at),
                timestamp(project.updated_at),
            ],
        )?;
        Ok(project)
    }

    pub fn list_projects(&self, include_archived: bool) -> HarnessResult<Vec<Project>> {
        let mut statement = self.connection.prepare(
            "SELECT id, name, description, workspace_path, instructions, routing_mode, status,
                    tags_json, created_at, updated_at
             FROM projects
             WHERE (?1 = 1 OR status <> 'archived')
             ORDER BY updated_at DESC, name COLLATE NOCASE",
        )?;
        let rows = statement.query_map([include_archived], project_from_row)?;
        collect_rows(rows)
    }

    pub fn project(&self, id: &str) -> HarnessResult<Project> {
        self.connection
            .query_row(
                "SELECT id, name, description, workspace_path, instructions, routing_mode, status,
                        tags_json, created_at, updated_at
                 FROM projects WHERE id = ?1",
                [id],
                project_from_row,
            )
            .optional()?
            .ok_or_else(|| HarnessError::NotFound(format!("project {id}")))
    }

    pub fn create_chat_group(&mut self, input: NewChatGroup) -> HarnessResult<ChatGroup> {
        ensure_exists(&self.connection, "projects", &input.project_id, "project")?;
        let name = required_text("group name", &input.name, 200)?;
        let sort_order: i64 = self.connection.query_row(
            "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM chat_groups WHERE project_id = ?1",
            [&input.project_id],
            |row| row.get(0),
        )?;
        let now = Utc::now();
        let group = ChatGroup {
            id: new_id(),
            project_id: input.project_id,
            name,
            sort_order,
            collapsed: false,
            archived: false,
            created_at: now,
            updated_at: now,
        };
        self.connection.execute(
            "INSERT INTO chat_groups
             (id, project_id, name, sort_order, collapsed, archived, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                group.id,
                group.project_id,
                group.name,
                group.sort_order,
                group.collapsed,
                group.archived,
                timestamp(group.created_at),
                timestamp(group.updated_at),
            ],
        )?;
        Ok(group)
    }

    pub fn list_chat_groups(&self, project_id: &str) -> HarnessResult<Vec<ChatGroup>> {
        let mut statement = self.connection.prepare(
            "SELECT id, project_id, name, sort_order, collapsed, archived, created_at, updated_at
             FROM chat_groups WHERE project_id = ?1
             ORDER BY archived, sort_order, name COLLATE NOCASE",
        )?;
        let rows = statement.query_map([project_id], |row| {
            Ok(ChatGroup {
                id: row.get(0)?,
                project_id: row.get(1)?,
                name: row.get(2)?,
                sort_order: row.get(3)?,
                collapsed: row.get(4)?,
                archived: row.get(5)?,
                created_at: datetime_column(row, 6)?,
                updated_at: datetime_column(row, 7)?,
            })
        })?;
        collect_rows(rows)
    }

    pub fn create_conversation(&mut self, input: NewConversation) -> HarnessResult<Conversation> {
        ensure_exists(&self.connection, "projects", &input.project_id, "project")?;
        if let Some(group_id) = input.group_id.as_deref() {
            let owner: Option<String> = self
                .connection
                .query_row(
                    "SELECT project_id FROM chat_groups WHERE id = ?1",
                    [group_id],
                    |row| row.get(0),
                )
                .optional()?;
            match owner {
                None => return Err(HarnessError::NotFound(format!("chat group {group_id}"))),
                Some(owner) if owner != input.project_id => {
                    return Err(HarnessError::Validation(
                        "chat group belongs to a different project".into(),
                    ))
                }
                _ => {}
            }
        }
        let now = Utc::now();
        let conversation = Conversation {
            id: new_id(),
            project_id: input.project_id,
            group_id: input.group_id,
            title: required_text("conversation title", &input.title, 300)?,
            status: ConversationStatus::Active,
            favorite: false,
            archived: false,
            tags: normalize_tags(input.tags)?,
            provider_policy: nonempty_optional(input.provider_policy),
            workspace_path: nonempty_optional(input.workspace_path),
            created_at: now,
            updated_at: now,
        };
        self.connection.execute(
            "INSERT INTO conversations
             (id, project_id, group_id, title, status, favorite, archived, tags_json,
              provider_policy, workspace_path, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                conversation.id,
                conversation.project_id,
                conversation.group_id,
                conversation.title,
                enum_key(&conversation.status)?,
                conversation.favorite,
                conversation.archived,
                serde_json::to_string(&conversation.tags)?,
                conversation.provider_policy,
                conversation.workspace_path,
                timestamp(conversation.created_at),
                timestamp(conversation.updated_at),
            ],
        )?;
        Ok(conversation)
    }

    pub fn list_conversations(
        &self,
        project_id: &str,
        include_archived: bool,
    ) -> HarnessResult<Vec<Conversation>> {
        let mut statement = self.connection.prepare(
            "SELECT id, project_id, group_id, title, status, favorite, archived, tags_json,
                    provider_policy, workspace_path, created_at, updated_at
             FROM conversations
             WHERE project_id = ?1 AND (?2 = 1 OR archived = 0)
             ORDER BY favorite DESC, updated_at DESC",
        )?;
        let rows =
            statement.query_map(params![project_id, include_archived], conversation_from_row)?;
        collect_rows(rows)
    }

    pub fn append_message(&mut self, input: NewMessage) -> HarnessResult<Message> {
        ensure_exists(
            &self.connection,
            "conversations",
            &input.conversation_id,
            "conversation",
        )?;
        if input.content.len() > 10 * 1024 * 1024 {
            return Err(HarnessError::Validation(
                "message exceeds the 10 MiB storage limit".into(),
            ));
        }
        if let Some(parent_id) = input.parent_message_id.as_deref() {
            ensure_exists(&self.connection, "messages", parent_id, "parent message")?;
        }
        let suggested_title =
            matches!(input.role, MessageRole::User).then(|| concise_title(&input.content));
        let message = Message {
            id: new_id(),
            conversation_id: input.conversation_id,
            role: input.role,
            content: input.content,
            provider_id: nonempty_optional(input.provider_id),
            model_id: nonempty_optional(input.model_id),
            parent_message_id: input.parent_message_id,
            metadata: input.metadata,
            created_at: Utc::now(),
        };
        let transaction = self.connection.transaction()?;
        transaction.execute(
            "INSERT INTO messages
             (id, conversation_id, role, content, provider_id, model_id, parent_message_id,
              metadata_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                message.id,
                message.conversation_id,
                enum_key(&message.role)?,
                message.content,
                message.provider_id,
                message.model_id,
                message.parent_message_id,
                serde_json::to_string(&message.metadata)?,
                timestamp(message.created_at),
            ],
        )?;
        transaction.execute(
            "UPDATE conversations
             SET updated_at = ?1,
                 title = CASE
                   WHEN title = 'Untitled conversation' AND COALESCE(?3, '') <> '' THEN ?3
                   ELSE title
                 END
             WHERE id = ?2",
            params![
                timestamp(message.created_at),
                message.conversation_id,
                suggested_title
            ],
        )?;
        transaction.commit()?;
        Ok(message)
    }

    pub fn list_messages(&self, conversation_id: &str) -> HarnessResult<Vec<Message>> {
        let mut statement = self.connection.prepare(
            "SELECT id, conversation_id, role, content, provider_id, model_id,
                    parent_message_id, metadata_json, created_at
             FROM messages WHERE conversation_id = ?1 ORDER BY created_at, rowid",
        )?;
        let rows = statement.query_map([conversation_id], message_from_row)?;
        collect_rows(rows)
    }

    pub fn search_messages(
        &self,
        query: &str,
        project_id: Option<&str>,
        limit: usize,
    ) -> HarnessResult<Vec<SearchHit>> {
        let query = fts_query(query)?;
        let limit = limit.clamp(1, 100) as i64;
        let mut statement = self.connection.prepare(
            "SELECT m.id, m.conversation_id, c.project_id, c.title, m.role,
                    snippet(messages_fts, 0, '[', ']', ' … ', 24),
                    bm25(messages_fts), m.created_at
             FROM messages_fts
             JOIN messages m ON m.rowid = messages_fts.rowid
             JOIN conversations c ON c.id = m.conversation_id
             WHERE messages_fts MATCH ?1 AND (?2 IS NULL OR c.project_id = ?2)
             ORDER BY bm25(messages_fts), m.created_at DESC
             LIMIT ?3",
        )?;
        let rows = statement.query_map(params![query, project_id, limit], |row| {
            Ok(SearchHit {
                message_id: row.get(0)?,
                conversation_id: row.get(1)?,
                project_id: row.get(2)?,
                conversation_title: row.get(3)?,
                role: json_enum_column(row, 4)?,
                snippet: row.get(5)?,
                rank: row.get(6)?,
                created_at: datetime_column(row, 7)?,
            })
        })?;
        collect_rows(rows)
    }

    pub fn persist_provider_snapshots(
        &mut self,
        snapshots: &[ProviderSnapshot],
    ) -> HarnessResult<()> {
        let transaction = self.connection.transaction()?;
        for provider in snapshots {
            transaction.execute(
                "INSERT INTO providers
                 (id, name, provider_type, enabled, status, version, capabilities_json,
                  status_detail, last_detected_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
                 ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    provider_type = excluded.provider_type,
                    enabled = excluded.enabled,
                    status = excluded.status,
                    version = excluded.version,
                    capabilities_json = excluded.capabilities_json,
                    status_detail = excluded.status_detail,
                    last_detected_at = excluded.last_detected_at",
                params![
                    provider.id,
                    provider.name,
                    enum_key(&provider.kind)?,
                    provider.enabled,
                    enum_key(&provider.status)?,
                    provider.version,
                    serde_json::to_string(&provider.capabilities)?,
                    provider.status_detail,
                    timestamp(provider.detected_at),
                ],
            )?;
            transaction.execute("DELETE FROM models WHERE provider_id = ?1", [&provider.id])?;
            for model in &provider.models {
                transaction.execute(
                    "INSERT INTO models
                     (id, provider_id, display_name, version, capabilities_json,
                      context_length, is_local)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        model.id,
                        provider.id,
                        model.display_name,
                        model.version,
                        serde_json::to_string(&model.capabilities)?,
                        model.context_length,
                        model.local,
                    ],
                )?;
            }
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn persist_task_graph(&mut self, tasks: &[TaskSpec]) -> HarnessResult<Vec<TaskRecord>> {
        let transaction = self.connection.transaction()?;
        let now = Utc::now();
        for task in tasks {
            transaction.execute(
                "INSERT INTO tasks
                 (id, parent_task_id, conversation_id, role, description, status, priority,
                  capabilities_json, preferred_provider, assigned_provider, assigned_model,
                  retry_count, max_retries, created_at)
                 VALUES (?1, NULL, ?2, ?3, ?4, 'pending', ?5, ?6, ?7, ?8, ?9, 0, ?10, ?11)",
                params![
                    task.id,
                    task.conversation_id,
                    task.role,
                    task.description,
                    task.priority,
                    serde_json::to_string(&task.capabilities)?,
                    task.preferred_provider,
                    task.assigned_provider,
                    task.assigned_model,
                    task.max_retries,
                    timestamp(now),
                ],
            )?;
        }
        for task in tasks {
            if let Some(parent) = &task.parent_task_id {
                transaction.execute(
                    "UPDATE tasks SET parent_task_id = ?1 WHERE id = ?2",
                    params![parent, task.id],
                )?;
            }
            for dependency in &task.dependencies {
                transaction.execute(
                    "INSERT INTO task_dependencies(task_id, depends_on_task_id) VALUES (?1, ?2)",
                    params![task.id, dependency],
                )?;
            }
        }
        transaction.commit()?;
        Ok(tasks
            .iter()
            .cloned()
            .map(|spec| TaskRecord {
                spec,
                status: TaskStatus::Pending,
                retry_count: 0,
                created_at: now,
                started_at: None,
                completed_at: None,
            })
            .collect())
    }

    pub fn create_agent_run(
        &mut self,
        task_id: &str,
        parent_run_id: Option<&str>,
        provider_id: &str,
        model_id: Option<&str>,
        attempt: u32,
    ) -> HarnessResult<String> {
        ensure_exists(&self.connection, "tasks", task_id, "task")?;
        let id = new_id();
        self.connection.execute(
            "INSERT INTO agent_runs
             (id, task_id, parent_run_id, provider_id, model_id, status, attempt, started_at)
             VALUES (?1, ?2, ?3, ?4, ?5, 'running', ?6, ?7)",
            params![
                id,
                task_id,
                parent_run_id,
                provider_id,
                model_id,
                attempt,
                timestamp(Utc::now())
            ],
        )?;
        Ok(id)
    }

    pub fn record_agent_event(&mut self, event: &AgentEvent) -> HarnessResult<()> {
        self.connection.execute(
            "INSERT INTO agent_events
             (id, run_id, parent_run_id, task_id, event_kind, sequence, payload_json, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                event.id,
                event.run_id,
                event.parent_run_id,
                event.task_id,
                enum_key(&event.kind)?,
                event.sequence,
                serde_json::to_string(&event.payload)?,
                timestamp(event.created_at),
            ],
        )?;
        Ok(())
    }

    pub fn cancel_agent_run(&mut self, run_id: &str) -> HarnessResult<()> {
        let changed = self.connection.execute(
            "UPDATE agent_runs
             SET status = 'cancelled', completed_at = ?1
             WHERE id = ?2 AND status IN ('queued', 'pending', 'running', 'waiting')",
            params![timestamp(Utc::now()), run_id],
        )?;
        if changed == 1 {
            return Ok(());
        }
        let status: Option<String> = self
            .connection
            .query_row(
                "SELECT status FROM agent_runs WHERE id = ?1",
                [run_id],
                |row| row.get(0),
            )
            .optional()?;
        match status {
            None => Err(HarnessError::NotFound(format!("agent run {run_id}"))),
            Some(status) => Err(HarnessError::Conflict(format!(
                "agent run {run_id} is already terminal ({status})"
            ))),
        }
    }

    pub fn record_usage(&mut self, event: &UsageEvent) -> HarnessResult<()> {
        let dimensions = &event.dimensions;
        self.connection.execute(
            "INSERT INTO usage_events
             (id, provider_id, model_id, project_id, conversation_id, task_id, agent_run_id,
              input_tokens, output_tokens, cached_tokens, reasoning_tokens, request_count,
              turn_count, tool_calls, duration_ms, estimated_cost, quota_units, local_tokens,
              local_compute_ms, tokens_per_second, peak_memory_bytes, raw_provider_usage_json,
              created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                     ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23)",
            params![
                event.id,
                event.provider_id,
                event.model_id,
                event.project_id,
                event.conversation_id,
                event.task_id,
                event.agent_run_id,
                dimensions.input_tokens,
                dimensions.output_tokens,
                dimensions.cached_tokens,
                dimensions.reasoning_tokens,
                dimensions.request_count,
                dimensions.turn_count,
                dimensions.tool_calls,
                dimensions.duration_ms,
                dimensions.estimated_cost,
                dimensions.quota_units,
                dimensions.local_tokens,
                dimensions.local_compute_ms,
                dimensions.tokens_per_second,
                dimensions.peak_memory_bytes,
                event
                    .raw_provider_usage
                    .as_ref()
                    .map(serde_json::to_string)
                    .transpose()?,
                timestamp(event.created_at),
            ],
        )?;
        Ok(())
    }

    pub fn usage_summary(
        &self,
        provider_id: Option<&str>,
        project_id: Option<&str>,
        range: Option<&DateRange>,
    ) -> HarnessResult<UsageSummary> {
        let start = range.and_then(|range| range.start.map(timestamp));
        let end = range.and_then(|range| range.end.map(timestamp));
        self.connection
            .query_row(
                "SELECT SUM(input_tokens), SUM(output_tokens), SUM(cached_tokens),
                        SUM(reasoning_tokens), SUM(request_count), SUM(tool_calls),
                        SUM(duration_ms), SUM(estimated_cost), SUM(local_compute_ms), COUNT(*)
                 FROM usage_events
                 WHERE (?1 IS NULL OR provider_id = ?1)
                   AND (?2 IS NULL OR project_id = ?2)
                   AND (?3 IS NULL OR created_at >= ?3)
                   AND (?4 IS NULL OR created_at < ?4)",
                params![provider_id, project_id, start, end],
                |row| {
                    Ok(UsageSummary {
                        provider_id: provider_id.map(str::to_owned),
                        input_tokens: optional_u64(row, 0)?,
                        output_tokens: optional_u64(row, 1)?,
                        cached_tokens: optional_u64(row, 2)?,
                        reasoning_tokens: optional_u64(row, 3)?,
                        request_count: optional_u64(row, 4)?,
                        tool_calls: optional_u64(row, 5)?,
                        duration_ms: optional_u64(row, 6)?,
                        estimated_cost: row.get(7)?,
                        local_compute_ms: optional_u64(row, 8)?,
                        event_count: row.get::<_, i64>(9)? as u64,
                    })
                },
            )
            .map_err(Into::into)
    }

    pub fn set_setting<T: Serialize>(
        &mut self,
        scope: &str,
        key: &str,
        value: &T,
    ) -> HarnessResult<()> {
        let scope = required_text("settings scope", scope, 100)?;
        let key = required_text("settings key", key, 200)?;
        self.connection.execute(
            "INSERT INTO settings(scope, key, value_json, updated_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(scope, key) DO UPDATE SET
                value_json = excluded.value_json,
                updated_at = excluded.updated_at",
            params![
                scope,
                key,
                serde_json::to_string(value)?,
                timestamp(Utc::now())
            ],
        )?;
        Ok(())
    }

    pub fn get_setting<T: DeserializeOwned>(
        &self,
        scope: &str,
        key: &str,
    ) -> HarnessResult<Option<T>> {
        let value: Option<String> = self
            .connection
            .query_row(
                "SELECT value_json FROM settings WHERE scope = ?1 AND key = ?2",
                params![scope, key],
                |row| row.get(0),
            )
            .optional()?;
        value
            .map(|value| serde_json::from_str(&value))
            .transpose()
            .map_err(Into::into)
    }

    pub fn table_count(&self, table: &str) -> HarnessResult<u64> {
        const ALLOWED: &[&str] = &[
            "projects",
            "chat_groups",
            "conversations",
            "messages",
            "providers",
            "models",
            "tasks",
            "agent_runs",
            "agent_events",
            "usage_events",
            "components",
            "workers",
        ];
        if !ALLOWED.contains(&table) {
            return Err(HarnessError::Validation("table is not queryable".into()));
        }
        let sql = format!("SELECT COUNT(*) FROM {table}");
        let count: i64 = self.connection.query_row(&sql, [], |row| row.get(0))?;
        Ok(count as u64)
    }
}

fn project_from_row(row: &Row<'_>) -> rusqlite::Result<Project> {
    Ok(Project {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        workspace_path: row.get(3)?,
        instructions: row.get(4)?,
        routing_mode: json_enum_column(row, 5)?,
        status: json_enum_column(row, 6)?,
        tags: json_column(row, 7)?,
        created_at: datetime_column(row, 8)?,
        updated_at: datetime_column(row, 9)?,
    })
}

fn conversation_from_row(row: &Row<'_>) -> rusqlite::Result<Conversation> {
    Ok(Conversation {
        id: row.get(0)?,
        project_id: row.get(1)?,
        group_id: row.get(2)?,
        title: row.get(3)?,
        status: json_enum_column(row, 4)?,
        favorite: row.get(5)?,
        archived: row.get(6)?,
        tags: json_column(row, 7)?,
        provider_policy: row.get(8)?,
        workspace_path: row.get(9)?,
        created_at: datetime_column(row, 10)?,
        updated_at: datetime_column(row, 11)?,
    })
}

fn message_from_row(row: &Row<'_>) -> rusqlite::Result<Message> {
    Ok(Message {
        id: row.get(0)?,
        conversation_id: row.get(1)?,
        role: json_enum_column(row, 2)?,
        content: row.get(3)?,
        provider_id: row.get(4)?,
        model_id: row.get(5)?,
        parent_message_id: row.get(6)?,
        metadata: json_column(row, 7)?,
        created_at: datetime_column(row, 8)?,
    })
}

fn required_text(label: &str, value: &str, max_len: usize) -> HarnessResult<String> {
    let value = value.trim();
    if value.is_empty() {
        return Err(HarnessError::Validation(format!("{label} is required")));
    }
    if value.chars().count() > max_len {
        return Err(HarnessError::Validation(format!(
            "{label} exceeds {max_len} characters"
        )));
    }
    Ok(value.to_owned())
}

fn nonempty_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let value = value.trim().to_owned();
        (!value.is_empty()).then_some(value)
    })
}

fn normalize_tags(tags: Vec<String>) -> HarnessResult<Vec<String>> {
    let mut unique = BTreeSet::new();
    for tag in tags {
        let tag = tag.trim();
        if tag.is_empty() {
            continue;
        }
        if tag.chars().count() > 80 {
            return Err(HarnessError::Validation("tag exceeds 80 characters".into()));
        }
        unique.insert(tag.to_owned());
    }
    if unique.len() > 50 {
        return Err(HarnessError::Validation(
            "a record may contain at most 50 tags".into(),
        ));
    }
    Ok(unique.into_iter().collect())
}

fn concise_title(value: &str) -> String {
    const MAX: usize = 80;
    let single_line = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut title: String = single_line.chars().take(MAX).collect();
    if single_line.chars().count() > MAX {
        title.push('…');
    }
    title
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

fn datetime_column(row: &Row<'_>, index: usize) -> rusqlite::Result<DateTime<Utc>> {
    let value: String = row.get(index)?;
    DateTime::parse_from_rfc3339(&value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(error))
        })
}

fn json_column<T: DeserializeOwned>(row: &Row<'_>, index: usize) -> rusqlite::Result<T> {
    let value: String = row.get(index)?;
    serde_json::from_str(&value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(error))
    })
}

fn json_enum_column<T: DeserializeOwned>(row: &Row<'_>, index: usize) -> rusqlite::Result<T> {
    let value: String = row.get(index)?;
    serde_json::from_value(serde_json::Value::String(value)).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(error))
    })
}

fn enum_key<T: Serialize>(value: &T) -> HarnessResult<String> {
    match serde_json::to_value(value)? {
        serde_json::Value::String(value) => Ok(value),
        _ => Err(HarnessError::Validation(
            "enum did not serialize to a string".into(),
        )),
    }
}

fn ensure_exists(connection: &Connection, table: &str, id: &str, label: &str) -> HarnessResult<()> {
    const ALLOWED: &[&str] = &[
        "projects",
        "chat_groups",
        "conversations",
        "messages",
        "tasks",
    ];
    if !ALLOWED.contains(&table) {
        return Err(HarnessError::Validation("invalid existence check".into()));
    }
    let sql = format!("SELECT 1 FROM {table} WHERE id = ?1");
    let exists = connection
        .query_row(&sql, [id], |_| Ok(()))
        .optional()?
        .is_some();
    if exists {
        Ok(())
    } else {
        Err(HarnessError::NotFound(format!("{label} {id}")))
    }
}

fn fts_query(query: &str) -> HarnessResult<String> {
    let terms: Vec<String> = query
        .split_whitespace()
        .filter_map(|term| {
            let term = term.replace('"', "");
            (!term.is_empty()).then(|| format!("\"{term}\""))
        })
        .take(20)
        .collect();
    if terms.is_empty() {
        return Err(HarnessError::Validation(
            "search query cannot be empty".into(),
        ));
    }
    Ok(terms.join(" AND "))
}

fn optional_u64(row: &Row<'_>, index: usize) -> rusqlite::Result<Option<u64>> {
    let value: Option<i64> = row.get(index)?;
    Ok(value.map(|value| value.max(0) as u64))
}

fn collect_rows<T>(
    rows: rusqlite::MappedRows<'_, impl FnMut(&Row<'_>) -> rusqlite::Result<T>>,
) -> HarnessResult<Vec<T>> {
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(db: &mut Database, name: &str) -> Project {
        db.create_project(NewProject {
            name: name.into(),
            description: String::new(),
            workspace_path: None,
            instructions: String::new(),
            tags: vec!["beta".into(), "alpha".into(), "alpha".into()],
            routing_mode: RoutingMode::Balanced,
        })
        .unwrap()
    }

    #[test]
    fn migrates_and_enforces_foreign_keys() {
        let db = Database::in_memory().unwrap();
        assert_eq!(db.schema_version().unwrap(), SCHEMA_VERSION);
        assert!(db.integrity_check().unwrap());
    }

    #[test]
    fn persists_project_group_conversation_and_searchable_messages() {
        let mut db = Database::in_memory().unwrap();
        let project = project(&mut db, "Harness");
        assert_eq!(project.tags, vec!["alpha", "beta"]);
        let group = db
            .create_chat_group(NewChatGroup {
                project_id: project.id.clone(),
                name: "Bugs".into(),
            })
            .unwrap();
        let conversation = db
            .create_conversation(NewConversation {
                project_id: project.id.clone(),
                group_id: Some(group.id),
                title: "Login failure".into(),
                tags: vec![],
                provider_policy: None,
                workspace_path: None,
            })
            .unwrap();
        db.append_message(NewMessage {
            conversation_id: conversation.id.clone(),
            role: MessageRole::User,
            content: "OAuth callback fails after login".into(),
            provider_id: None,
            model_id: None,
            parent_message_id: None,
            metadata: serde_json::json!({}),
        })
        .unwrap();

        let hits = db
            .search_messages("OAuth callback", Some(&project.id), 20)
            .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].conversation_title, "Login failure");
        assert!(hits[0].snippet.contains("[OAuth]"));
        assert_eq!(db.list_messages(&conversation.id).unwrap().len(), 1);
    }

    #[test]
    fn rejects_group_from_another_project() {
        let mut db = Database::in_memory().unwrap();
        let first = project(&mut db, "First");
        let second = project(&mut db, "Second");
        let group = db
            .create_chat_group(NewChatGroup {
                project_id: first.id,
                name: "Group".into(),
            })
            .unwrap();
        let result = db.create_conversation(NewConversation {
            project_id: second.id,
            group_id: Some(group.id),
            title: "Invalid".into(),
            tags: vec![],
            provider_policy: None,
            workspace_path: None,
        });
        assert!(matches!(result, Err(HarnessError::Validation(_))));
    }

    #[test]
    fn first_user_message_titles_an_untitled_conversation() {
        let mut db = Database::in_memory().unwrap();
        let project = project(&mut db, "Harness");
        let conversation = db
            .create_conversation(NewConversation {
                project_id: project.id.clone(),
                group_id: None,
                title: "Untitled conversation".into(),
                tags: vec![],
                provider_policy: None,
                workspace_path: None,
            })
            .unwrap();

        db.append_message(NewMessage {
            conversation_id: conversation.id,
            role: MessageRole::User,
            content: "  Diagnose the login callback\nwithout changing production  ".into(),
            provider_id: None,
            model_id: None,
            parent_message_id: None,
            metadata: serde_json::json!({}),
        })
        .unwrap();

        let conversations = db.list_conversations(&project.id, false).unwrap();
        assert_eq!(
            conversations[0].title,
            "Diagnose the login callback without changing production"
        );
    }

    #[test]
    fn usage_preserves_unknown_values_as_null() {
        let mut db = Database::in_memory().unwrap();
        db.record_usage(&UsageEvent {
            id: new_id(),
            provider_id: "m365".into(),
            model_id: None,
            project_id: None,
            conversation_id: None,
            task_id: None,
            agent_run_id: None,
            dimensions: UsageDimensions {
                request_count: Some(2),
                ..UsageDimensions::default()
            },
            raw_provider_usage: None,
            created_at: Utc::now(),
        })
        .unwrap();
        let summary = db.usage_summary(Some("m365"), None, None).unwrap();
        assert_eq!(summary.request_count, Some(2));
        assert_eq!(summary.input_tokens, None);
        assert_eq!(summary.estimated_cost, None);
    }

    #[test]
    fn settings_round_trip_json_without_secret_assumptions() {
        let mut db = Database::in_memory().unwrap();
        db.set_setting("routing", "mode", &RoutingMode::LocalFirst)
            .unwrap();
        let value: Option<RoutingMode> = db.get_setting("routing", "mode").unwrap();
        assert_eq!(value, Some(RoutingMode::LocalFirst));
    }
}
