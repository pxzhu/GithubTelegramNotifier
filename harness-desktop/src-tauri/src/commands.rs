use crate::components::{plan_update, UpdateCandidate, UpdatePlan};
use crate::database::Database;
use crate::domain::*;
use crate::error::{CommandError, HarnessError};
use crate::hardware::{
    analyze_hardware as detect_hardware, recommend_local_ai, HardwareProfile, LocalAiRecommendation,
};
use crate::providers::{
    claude_invocation, codex_invocation, ExecutionAccess, ProviderManifest, ProviderRegistry,
    SafeCliInvocation, CLAUDE_PROVIDER_ID, CODEX_PROVIDER_ID,
};
use crate::routing::DeterministicRouter;
use crate::runtime::TaskGraph;
use crate::security::{
    evaluate_permission as evaluate_policy, PermissionDecision, PermissionPolicy, PermissionRequest,
};
use serde::Serialize;
use std::path::Path;
use std::sync::Mutex;
use tauri::State;

pub struct AppState {
    pub database: Mutex<Database>,
}

#[derive(Debug, Serialize)]
pub struct ExternalGate {
    pub id: &'static str,
    pub available: bool,
    pub reason: &'static str,
}

#[derive(Debug, Serialize)]
pub struct HealthSnapshot {
    pub app_version: &'static str,
    pub schema_version: i64,
    pub database_integrity: bool,
    pub local_first: bool,
    pub external_gates: Vec<ExternalGate>,
}

#[derive(Debug, Serialize)]
pub struct BootstrapSnapshot {
    pub health: HealthSnapshot,
    pub projects: Vec<Project>,
    pub providers: Vec<ProviderSnapshot>,
    pub hardware: HardwareProfile,
    pub local_ai_recommendation: LocalAiRecommendation,
}

#[tauri::command]
pub fn app_health(state: State<'_, AppState>) -> Result<HealthSnapshot, CommandError> {
    let database = state.database.lock()?;
    health_snapshot(&database)
}

#[tauri::command]
pub async fn bootstrap(state: State<'_, AppState>) -> Result<BootstrapSnapshot, CommandError> {
    let projects = {
        let database = state.database.lock()?;
        database.list_projects(false).map_err(CommandError::from)?
    };
    let providers = tauri::async_runtime::spawn_blocking(|| ProviderRegistry.detect_all())
        .await
        .map_err(|_| command_error("provider_probe_failed", "provider probe task failed"))?;
    {
        let mut database = state.database.lock()?;
        database
            .persist_provider_snapshots(&providers)
            .map_err(CommandError::from)?;
    }
    let hardware = tauri::async_runtime::spawn_blocking(detect_hardware)
        .await
        .map_err(|_| command_error("hardware_probe_failed", "hardware probe task failed"))?;
    let recommendation = recommend_local_ai(&hardware);
    let health = {
        let database = state.database.lock()?;
        health_snapshot(&database)?
    };
    Ok(BootstrapSnapshot {
        health,
        projects,
        providers,
        hardware,
        local_ai_recommendation: recommendation,
    })
}

#[tauri::command]
pub fn create_project(
    state: State<'_, AppState>,
    input: NewProject,
) -> Result<Project, CommandError> {
    state
        .database
        .lock()?
        .create_project(input)
        .map_err(Into::into)
}

#[tauri::command]
pub fn list_projects(
    state: State<'_, AppState>,
    include_archived: Option<bool>,
) -> Result<Vec<Project>, CommandError> {
    state
        .database
        .lock()?
        .list_projects(include_archived.unwrap_or(false))
        .map_err(Into::into)
}

#[tauri::command]
pub fn create_chat_group(
    state: State<'_, AppState>,
    input: NewChatGroup,
) -> Result<ChatGroup, CommandError> {
    state
        .database
        .lock()?
        .create_chat_group(input)
        .map_err(Into::into)
}

#[tauri::command]
pub fn list_chat_groups(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<Vec<ChatGroup>, CommandError> {
    state
        .database
        .lock()?
        .list_chat_groups(&project_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn create_conversation(
    state: State<'_, AppState>,
    input: NewConversation,
) -> Result<Conversation, CommandError> {
    state
        .database
        .lock()?
        .create_conversation(input)
        .map_err(Into::into)
}

#[tauri::command]
pub fn list_conversations(
    state: State<'_, AppState>,
    project_id: String,
    include_archived: Option<bool>,
) -> Result<Vec<Conversation>, CommandError> {
    state
        .database
        .lock()?
        .list_conversations(&project_id, include_archived.unwrap_or(false))
        .map_err(Into::into)
}

#[tauri::command]
pub fn append_message(
    state: State<'_, AppState>,
    input: NewMessage,
) -> Result<Message, CommandError> {
    state
        .database
        .lock()?
        .append_message(input)
        .map_err(Into::into)
}

#[tauri::command]
pub fn list_messages(
    state: State<'_, AppState>,
    conversation_id: String,
) -> Result<Vec<Message>, CommandError> {
    state
        .database
        .lock()?
        .list_messages(&conversation_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn search_messages(
    state: State<'_, AppState>,
    query: String,
    project_id: Option<String>,
    limit: Option<usize>,
) -> Result<Vec<SearchHit>, CommandError> {
    state
        .database
        .lock()?
        .search_messages(&query, project_id.as_deref(), limit.unwrap_or(30))
        .map_err(Into::into)
}

#[tauri::command]
pub async fn detect_providers(
    state: State<'_, AppState>,
) -> Result<Vec<ProviderSnapshot>, CommandError> {
    let snapshots = tauri::async_runtime::spawn_blocking(|| ProviderRegistry.detect_all())
        .await
        .map_err(|_| command_error("provider_probe_failed", "provider probe task failed"))?;
    state
        .database
        .lock()?
        .persist_provider_snapshots(&snapshots)
        .map_err(CommandError::from)?;
    Ok(snapshots)
}

#[tauri::command]
pub fn route_request(
    request: RouteRequest,
    providers: Vec<ProviderSnapshot>,
) -> Result<RouteDecision, CommandError> {
    DeterministicRouter::route(&request, &providers, None).map_err(Into::into)
}

#[tauri::command]
pub fn save_task_graph(
    state: State<'_, AppState>,
    tasks: Vec<TaskSpec>,
) -> Result<Vec<TaskRecord>, CommandError> {
    let graph = TaskGraph::new(tasks).map_err(CommandError::from)?;
    state
        .database
        .lock()?
        .persist_task_graph(&graph.tasks)
        .map_err(Into::into)
}

#[tauri::command]
pub fn record_usage(state: State<'_, AppState>, event: UsageEvent) -> Result<(), CommandError> {
    state
        .database
        .lock()?
        .record_usage(&event)
        .map_err(Into::into)
}

#[tauri::command]
pub fn usage_summary(
    state: State<'_, AppState>,
    provider_id: Option<String>,
    project_id: Option<String>,
    range: Option<DateRange>,
) -> Result<UsageSummary, CommandError> {
    state
        .database
        .lock()?
        .usage_summary(
            provider_id.as_deref(),
            project_id.as_deref(),
            range.as_ref(),
        )
        .map_err(Into::into)
}

#[tauri::command]
pub async fn analyze_hardware() -> Result<HardwareProfile, CommandError> {
    tauri::async_runtime::spawn_blocking(detect_hardware)
        .await
        .map_err(|_| command_error("hardware_probe_failed", "hardware probe task failed"))
}

#[tauri::command]
pub fn local_ai_recommendation(profile: HardwareProfile) -> LocalAiRecommendation {
    recommend_local_ai(&profile)
}

#[tauri::command]
pub fn validate_provider_manifest(json: String) -> Result<ProviderManifest, CommandError> {
    ProviderManifest::parse_and_validate(&json).map_err(Into::into)
}

#[tauri::command]
pub fn plan_component_update(candidate: UpdateCandidate) -> Result<UpdatePlan, CommandError> {
    plan_update(&candidate).map_err(Into::into)
}

#[tauri::command]
pub fn evaluate_permission(
    policy: PermissionPolicy,
    request: PermissionRequest,
) -> PermissionDecision {
    evaluate_policy(&policy, &request)
}

#[tauri::command]
pub fn plan_cli_invocation(
    provider_id: String,
    executable_path: String,
    workspace_path: String,
    access: ExecutionAccess,
    resume_session: Option<String>,
) -> Result<SafeCliInvocation, CommandError> {
    let executable = Path::new(&executable_path);
    let workspace = Path::new(&workspace_path);
    match provider_id.as_str() {
        CLAUDE_PROVIDER_ID => {
            claude_invocation(executable, workspace, access, resume_session.as_deref())
        }
        CODEX_PROVIDER_ID => {
            codex_invocation(executable, workspace, access, resume_session.as_deref())
        }
        _ => Err(HarnessError::Validation(
            "provider does not have a built-in constrained invocation plan".into(),
        )),
    }
    .map_err(Into::into)
}

fn health_snapshot(database: &Database) -> Result<HealthSnapshot, CommandError> {
    Ok(HealthSnapshot {
        app_version: env!("CARGO_PKG_VERSION"),
        schema_version: database.schema_version().map_err(CommandError::from)?,
        database_integrity: database.integrity_check().map_err(CommandError::from)?,
        local_first: true,
        external_gates: vec![
            ExternalGate {
                id: "m365_pkce_and_secure_store",
                available: false,
                reason: "Requires tenant configuration, delegated consent, a Copilot license, and an OS credential-store adapter",
            },
            ExternalGate {
                id: "signed_app_updater",
                available: false,
                reason: "Requires production release endpoints and signing public keys",
            },
            ExternalGate {
                id: "signed_model_catalog_downloads",
                available: false,
                reason: "Requires a signed catalog, license UX, download source, and release hashes",
            },
            ExternalGate {
                id: "distributed_worker_transport",
                available: false,
                reason: "Pairing policy exists, but the mutually authenticated TLS transport is not configured",
            },
        ],
    })
}

fn command_error(code: &'static str, message: &str) -> CommandError {
    CommandError {
        code,
        message: message.into(),
    }
}
