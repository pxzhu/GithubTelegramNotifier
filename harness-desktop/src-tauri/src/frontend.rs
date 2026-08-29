use crate::commands::AppState;
use crate::domain::{
    ConversationStatus, MessageRole, NewMessage, ProviderSnapshot, ProviderStatus, UsageSummary,
};
use crate::error::{CommandError, HarnessError};
use crate::hardware::{analyze_hardware, recommend_local_ai, HardwareTier};
use crate::providers::ProviderRegistry;
use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tauri::State;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendProject {
    id: String,
    name: String,
    description: String,
    color: String,
    workspace: String,
    branch: Option<String>,
    conversations: usize,
    updated_at: String,
    archived: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendChatGroup {
    id: String,
    project_id: String,
    name: String,
    collapsed: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendConversation {
    id: String,
    project_id: String,
    group_id: String,
    title: String,
    preview: String,
    updated_at: String,
    unread: bool,
    favorite: bool,
    archived: bool,
    status: &'static str,
    tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendProvider {
    id: &'static str,
    name: String,
    short_name: &'static str,
    model: String,
    status: &'static str,
    version: String,
    color: &'static str,
    capabilities: Vec<String>,
    source: &'static str,
    description: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendMessage {
    id: String,
    conversation_id: String,
    role: &'static str,
    author: &'static str,
    content: String,
    created_at: String,
    provider: Option<String>,
    model: Option<String>,
    duration: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendTaskRun {
    id: String,
    title: String,
    description: String,
    status: &'static str,
    provider: String,
    model: Option<String>,
    duration: Option<String>,
    parent_id: Option<String>,
    progress: Option<u8>,
    events: Vec<FrontendRunEvent>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrontendRunEvent {
    id: String,
    kind: &'static str,
    label: String,
    detail: Option<String>,
    timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct FrontendFileChange {
    path: String,
    additions: u64,
    deletions: u64,
    status: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendUsageDatum {
    provider: String,
    color: &'static str,
    percent: u8,
    requests: u64,
    tokens: Option<String>,
    cost: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendHardwareProfile {
    device: &'static str,
    cpu: String,
    memory: String,
    gpu: String,
    free_disk: String,
    tier: u8,
}

#[derive(Debug, Clone, Serialize)]
pub struct HarnessSnapshot {
    projects: Vec<FrontendProject>,
    groups: Vec<FrontendChatGroup>,
    conversations: Vec<FrontendConversation>,
    providers: Vec<FrontendProvider>,
    messages: Vec<FrontendMessage>,
    tasks: Vec<FrontendTaskRun>,
    files: Vec<FrontendFileChange>,
    usage: Vec<FrontendUsageDatum>,
    hardware: FrontendHardwareProfile,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComposerDraft {
    text: String,
    provider: String,
    routing_mode: String,
    #[serde(default)]
    attachments: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SendMessageResult {
    accepted: bool,
    dispatched: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendSearchResult {
    id: String,
    #[serde(rename = "type")]
    result_type: &'static str,
    title: String,
    excerpt: String,
    project: Option<String>,
    conversation_id: Option<String>,
    command: Option<String>,
}

#[tauri::command]
pub async fn get_harness_snapshot(
    state: State<'_, AppState>,
) -> Result<HarnessSnapshot, CommandError> {
    let providers = tauri::async_runtime::spawn_blocking(|| ProviderRegistry.detect_all())
        .await
        .map_err(|_| command_error("provider_probe_failed", "provider probe task failed"))?;
    let hardware = tauri::async_runtime::spawn_blocking(analyze_hardware)
        .await
        .map_err(|_| command_error("hardware_probe_failed", "hardware probe task failed"))?;

    let mut database = state.database.lock()?;
    database
        .persist_provider_snapshots(&providers)
        .map_err(CommandError::from)?;
    let projects = database.list_projects(true).map_err(CommandError::from)?;
    let mut all_groups = Vec::new();
    let mut all_conversations = Vec::new();
    let mut all_messages = Vec::new();
    let mut counts = HashMap::<String, usize>::new();
    for project in &projects {
        all_groups.extend(
            database
                .list_chat_groups(&project.id)
                .map_err(CommandError::from)?,
        );
        let conversations = database
            .list_conversations(&project.id, true)
            .map_err(CommandError::from)?;
        counts.insert(project.id.clone(), conversations.len());
        for conversation in &conversations {
            all_messages.extend(
                database
                    .list_messages(&conversation.id)
                    .map_err(CommandError::from)?,
            );
        }
        all_conversations.extend(conversations);
    }
    let mut usage_rows = Vec::new();
    for provider in &providers {
        usage_rows.push((
            provider,
            database
                .usage_summary(Some(&provider.id), None, None)
                .map_err(CommandError::from)?,
        ));
    }
    drop(database);

    let latest_message: HashMap<&str, &crate::domain::Message> = all_messages
        .iter()
        .map(|message| (message.conversation_id.as_str(), message))
        .collect();
    let total_requests: u64 = usage_rows
        .iter()
        .map(|(_, summary)| usage_request_count(summary))
        .sum();
    let recommendation = recommend_local_ai(&hardware);

    Ok(HarnessSnapshot {
        projects: projects
            .into_iter()
            .map(|project| FrontendProject {
                color: stable_color(&project.id),
                conversations: counts.get(&project.id).copied().unwrap_or(0),
                id: project.id,
                name: project.name,
                description: project.description,
                workspace: project.workspace_path.unwrap_or_default(),
                branch: None,
                updated_at: display_timestamp(&project.updated_at),
                archived: matches!(project.status, crate::domain::ProjectStatus::Archived),
            })
            .collect(),
        groups: all_groups
            .into_iter()
            .filter(|group| !group.archived)
            .map(|group| FrontendChatGroup {
                id: group.id,
                project_id: group.project_id,
                name: group.name,
                collapsed: group.collapsed,
            })
            .collect(),
        conversations: all_conversations
            .into_iter()
            .map(|conversation| FrontendConversation {
                preview: latest_message
                    .get(conversation.id.as_str())
                    .map(|message| truncate_preview(&message.content))
                    .unwrap_or_default(),
                status: frontend_conversation_status(&conversation.status),
                id: conversation.id,
                project_id: conversation.project_id,
                group_id: conversation.group_id.unwrap_or_default(),
                title: conversation.title,
                updated_at: display_timestamp(&conversation.updated_at),
                unread: false,
                favorite: conversation.favorite,
                archived: conversation.archived,
                tags: conversation.tags,
            })
            .collect(),
        providers: providers.iter().filter_map(frontend_provider).collect(),
        messages: all_messages.into_iter().map(frontend_message).collect(),
        // Task and file-change readers are intentionally empty until an actual run persists them.
        tasks: vec![],
        files: vec![],
        usage: usage_rows
            .into_iter()
            .filter_map(|(provider, summary)| {
                let requests = usage_request_count(&summary);
                (summary.event_count > 0).then(|| FrontendUsageDatum {
                    provider: provider.name.clone(),
                    color: provider_color(&provider.id),
                    percent: requests
                        .saturating_mul(100)
                        .checked_div(total_requests)
                        .unwrap_or(0)
                        .min(100) as u8,
                    requests,
                    tokens: token_label(&summary),
                    cost: summary.estimated_cost.map(|cost| format!("${cost:.2}")),
                })
            })
            .collect(),
        hardware: FrontendHardwareProfile {
            device: if cfg!(target_os = "macos") {
                "This Mac"
            } else {
                "This PC"
            },
            cpu: format!(
                "{} · {} logical cores",
                hardware.cpu_brand, hardware.logical_cpu_count
            ),
            memory: byte_label(hardware.total_memory_bytes),
            gpu: hardware
                .gpu
                .first()
                .map(|gpu| {
                    format!(
                        "{} · {}",
                        gpu.model,
                        gpu.backend.as_deref().unwrap_or("backend unknown")
                    )
                })
                .unwrap_or_else(|| "No supported GPU confirmed".into()),
            free_disk: format!("{} available", byte_label(hardware.available_disk_bytes)),
            tier: match recommendation.tier {
                HardwareTier::Tier0 => 0,
                HardwareTier::Tier1 => 1,
                HardwareTier::Tier2 => 2,
                HardwareTier::Tier3 => 3,
                HardwareTier::Tier4 => 4,
            },
        },
    })
}

#[tauri::command(rename_all = "camelCase")]
pub fn send_message(
    state: State<'_, AppState>,
    conversation_id: String,
    draft: ComposerDraft,
) -> Result<SendMessageResult, CommandError> {
    if draft.text.trim().is_empty() {
        return Err(HarnessError::Validation("message text is required".into()).into());
    }
    let metadata = serde_json::json!({
        "requested_provider": draft.provider,
        "routing_mode": draft.routing_mode,
        "attachments": draft.attachments,
        "dispatch_state": "persisted_not_dispatched"
    });
    state
        .database
        .lock()?
        .append_message(NewMessage {
            conversation_id,
            role: MessageRole::User,
            content: draft.text,
            provider_id: None,
            model_id: None,
            parent_message_id: None,
            metadata,
        })
        .map_err(CommandError::from)?;
    Ok(SendMessageResult {
        accepted: true,
        dispatched: false,
    })
}

#[tauri::command(rename_all = "camelCase")]
pub fn cancel_agent_run(state: State<'_, AppState>, run_id: String) -> Result<(), CommandError> {
    state
        .database
        .lock()?
        .cancel_agent_run(&run_id)
        .map_err(Into::into)
}

#[tauri::command]
pub fn search_harness(
    state: State<'_, AppState>,
    query: String,
) -> Result<Vec<FrontendSearchResult>, CommandError> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }
    let database = state.database.lock()?;
    let projects: HashMap<String, String> = database
        .list_projects(true)
        .map_err(CommandError::from)?
        .into_iter()
        .map(|project| (project.id, project.name))
        .collect();
    database
        .search_messages(&query, None, 12)
        .map_err(CommandError::from)
        .map(|hits| {
            hits.into_iter()
                .map(|hit| FrontendSearchResult {
                    id: format!("message-{}", hit.message_id),
                    result_type: "message",
                    title: hit.conversation_title,
                    excerpt: hit.snippet,
                    project: projects.get(&hit.project_id).cloned(),
                    conversation_id: Some(hit.conversation_id),
                    command: None,
                })
                .collect()
        })
}

#[tauri::command]
pub fn choose_workspace() -> Option<String> {
    // The dialog plugin is intentionally not granted. Returning null is honest and prevents
    // native builds from falling through to a demo filesystem path.
    None
}

fn frontend_provider(provider: &ProviderSnapshot) -> Option<FrontendProvider> {
    let (id, short_name, color, source) = match provider.id.as_str() {
        "anthropic.claude-code" => ("claude", "CL", "#d97757", "Vendor CLI session"),
        "openai.codex-cli" => ("codex", "CX", "#20262e", "Vendor CLI session"),
        "microsoft.m365-copilot" => ("m365", "M", "#4f75e8", "Built-in preview adapter"),
        _ => return None,
    };
    Some(FrontendProvider {
        id,
        name: provider.name.clone(),
        short_name,
        model: provider
            .models
            .first()
            .map(|model| model.display_name.clone())
            .unwrap_or_else(|| "Provider managed".into()),
        status: match provider.status {
            ProviderStatus::Connected => "connected",
            ProviderStatus::Available | ProviderStatus::Installed => "ready",
            ProviderStatus::LoginRequired => "login-required",
            ProviderStatus::UpdateAvailable => "update",
            _ => "unavailable",
        },
        version: provider.version.clone().unwrap_or_else(|| "Unknown".into()),
        color,
        capabilities: provider
            .capabilities
            .iter()
            .map(|capability| format!("{capability:?}"))
            .collect(),
        source,
        description: provider.status_detail.clone().unwrap_or_default(),
    })
}

fn frontend_message(message: crate::domain::Message) -> FrontendMessage {
    let (role, author) = match message.role {
        MessageRole::User => ("user", "You"),
        MessageRole::System => ("system", "System"),
        MessageRole::Assistant | MessageRole::Agent | MessageRole::Tool => ("assistant", "Harness"),
    };
    FrontendMessage {
        id: message.id,
        conversation_id: message.conversation_id,
        role,
        author,
        content: message.content,
        created_at: display_timestamp(&message.created_at),
        provider: message.provider_id,
        model: message.model_id,
        duration: None,
    }
}

fn frontend_conversation_status(status: &ConversationStatus) -> &'static str {
    match status {
        ConversationStatus::Running => "working",
        ConversationStatus::Failed => "attention",
        _ => "idle",
    }
}

fn usage_request_count(summary: &UsageSummary) -> u64 {
    summary.request_count.unwrap_or(summary.event_count)
}

fn token_label(summary: &UsageSummary) -> Option<String> {
    match (summary.input_tokens, summary.output_tokens) {
        (None, None) => None,
        (input, output) => Some(format!(
            "{} tokens",
            input.unwrap_or(0).saturating_add(output.unwrap_or(0))
        )),
    }
}

fn truncate_preview(value: &str) -> String {
    const MAX: usize = 180;
    let mut preview: String = value.chars().take(MAX).collect();
    if value.chars().count() > MAX {
        preview.push('…');
    }
    preview
}

fn display_timestamp(value: &DateTime<Utc>) -> String {
    let local = value.with_timezone(&Local);
    if local.date_naive() == Local::now().date_naive() {
        local.format("%H:%M").to_string()
    } else {
        local.format("%Y-%m-%d").to_string()
    }
}

fn byte_label(bytes: u64) -> String {
    format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
}

fn stable_color(id: &str) -> String {
    const COLORS: &[&str] = &["#5b7cfa", "#23a77b", "#c675d1", "#e69735", "#d97757"];
    let hash = id.bytes().fold(2_166_136_261_u32, |hash, byte| {
        (hash ^ byte as u32).wrapping_mul(16_777_619)
    });
    COLORS[hash as usize % COLORS.len()].into()
}

fn provider_color(id: &str) -> &'static str {
    match id {
        "anthropic.claude-code" => "#d97757",
        "microsoft.m365-copilot" => "#4f75e8",
        "openai.codex-cli" => "#20262e",
        _ => "#657180",
    }
}

fn command_error(code: &'static str, message: &str) -> CommandError {
    CommandError {
        code,
        message: message.into(),
    }
}
