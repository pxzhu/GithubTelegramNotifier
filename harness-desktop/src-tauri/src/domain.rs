use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use uuid::Uuid;

pub fn new_id() -> String {
    Uuid::new_v4().to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    Active,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub description: String,
    pub workspace_path: Option<String>,
    pub instructions: String,
    pub routing_mode: RoutingMode,
    pub status: ProjectStatus,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewProject {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub workspace_path: Option<String>,
    #[serde(default)]
    pub instructions: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub routing_mode: RoutingMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatGroup {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub sort_order: i64,
    pub collapsed: bool,
    pub archived: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewChatGroup {
    pub project_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConversationStatus {
    Active,
    Running,
    Failed,
    Completed,
    Archived,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub project_id: String,
    pub group_id: Option<String>,
    pub title: String,
    pub status: ConversationStatus,
    pub favorite: bool,
    pub archived: bool,
    pub tags: Vec<String>,
    pub provider_policy: Option<String>,
    pub workspace_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewConversation {
    pub project_id: String,
    pub group_id: Option<String>,
    pub title: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub provider_policy: Option<String>,
    pub workspace_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Agent,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub conversation_id: String,
    pub role: MessageRole,
    pub content: String,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    pub parent_message_id: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewMessage {
    pub conversation_id: String,
    pub role: MessageRole,
    pub content: String,
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    pub parent_message_id: Option<String>,
    #[serde(default)]
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub message_id: String,
    pub conversation_id: String,
    pub project_id: String,
    pub conversation_title: String,
    pub role: MessageRole,
    pub snippet: String,
    pub rank: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Coding,
    Reasoning,
    CompanySearch,
    FileAccess,
    Shell,
    Git,
    ToolCalling,
    Vision,
    LongContext,
    Local,
    Offline,
    Cheap,
    Fast,
    ParallelAgents,
    SessionResume,
    TextGeneration,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    Cli,
    Http,
    LocalRuntime,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderStatus {
    Available,
    Installed,
    Connected,
    LoginRequired,
    UpdateAvailable,
    Disabled,
    Unavailable,
    PermissionUnavailable,
    Error,
}

impl ProviderStatus {
    pub fn can_execute(self) -> bool {
        matches!(self, Self::Connected | Self::Available)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelDescriptor {
    pub id: String,
    pub display_name: String,
    pub capabilities: BTreeSet<Capability>,
    pub context_length: Option<u64>,
    pub local: bool,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSnapshot {
    pub id: String,
    pub name: String,
    pub kind: ProviderKind,
    pub status: ProviderStatus,
    pub enabled: bool,
    pub version: Option<String>,
    pub capabilities: BTreeSet<Capability>,
    pub models: Vec<ModelDescriptor>,
    pub status_detail: Option<String>,
    pub executable_path: Option<String>,
    pub detected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum RoutingMode {
    QualityFirst,
    #[default]
    Balanced,
    ClaudeSaver,
    LocalFirst,
    Custom,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Complexity {
    Low,
    Medium,
    High,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteRequest {
    pub explicit_provider: Option<String>,
    #[serde(default)]
    pub required_capabilities: BTreeSet<Capability>,
    #[serde(default)]
    pub requires_company_context: bool,
    #[serde(default)]
    pub modifies_repository: bool,
    #[serde(default)]
    pub offline_requested: bool,
    pub complexity: Complexity,
    #[serde(default)]
    pub mode: RoutingMode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteDecision {
    pub provider_id: Option<String>,
    pub model_id: Option<String>,
    pub confidence: f32,
    pub planner_required: bool,
    pub reasons: Vec<String>,
    pub rejected: Vec<ProviderRejection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderRejection {
    pub provider_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Ready,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskSpec {
    pub id: String,
    pub parent_task_id: Option<String>,
    pub conversation_id: String,
    pub role: String,
    pub description: String,
    pub priority: i32,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub capabilities: BTreeSet<Capability>,
    pub preferred_provider: Option<String>,
    pub assigned_provider: Option<String>,
    pub assigned_model: Option<String>,
    #[serde(default)]
    pub max_retries: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRecord {
    pub spec: TaskSpec,
    pub status: TaskStatus,
    pub retry_count: u32,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentEventKind {
    Started,
    TextDelta,
    ToolStarted,
    ToolCompleted,
    SubagentStarted,
    SubagentCompleted,
    Usage,
    Retry,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    pub id: String,
    pub run_id: String,
    pub parent_run_id: Option<String>,
    pub task_id: String,
    pub kind: AgentEventKind,
    pub sequence: u64,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsageDimensions {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cached_tokens: Option<u64>,
    pub reasoning_tokens: Option<u64>,
    pub request_count: Option<u64>,
    pub turn_count: Option<u64>,
    pub tool_calls: Option<u64>,
    pub duration_ms: Option<u64>,
    pub estimated_cost: Option<f64>,
    pub quota_units: Option<f64>,
    pub local_tokens: Option<u64>,
    pub local_compute_ms: Option<u64>,
    pub tokens_per_second: Option<f64>,
    pub peak_memory_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageEvent {
    pub id: String,
    pub provider_id: String,
    pub model_id: Option<String>,
    pub project_id: Option<String>,
    pub conversation_id: Option<String>,
    pub task_id: Option<String>,
    pub agent_run_id: Option<String>,
    pub dimensions: UsageDimensions,
    pub raw_provider_usage: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UsageSummary {
    pub provider_id: Option<String>,
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cached_tokens: Option<u64>,
    pub reasoning_tokens: Option<u64>,
    pub request_count: Option<u64>,
    pub tool_calls: Option<u64>,
    pub duration_ms: Option<u64>,
    pub estimated_cost: Option<f64>,
    pub local_compute_ms: Option<u64>,
    pub event_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}
