export type ProviderId = "auto" | "claude" | "m365" | "local" | "codex";

export type Screen = "workspace" | "search" | "usage" | "settings";
export type InspectorTab = "activity" | "usage" | "context" | "files" | "details";
export type RoutingMode = "balanced" | "quality" | "claude-saver" | "local-first";
export type RunStatus = "queued" | "running" | "completed" | "failed" | "waiting";

export interface Project {
  id: string;
  name: string;
  description: string;
  color: string;
  workspace: string;
  branch?: string;
  conversations: number;
  updatedAt: string;
  archived?: boolean;
}

export interface ChatGroup {
  id: string;
  projectId: string;
  name: string;
  collapsed?: boolean;
}

export interface Conversation {
  id: string;
  projectId: string;
  groupId: string;
  title: string;
  preview: string;
  updatedAt: string;
  unread?: boolean;
  favorite?: boolean;
  archived?: boolean;
  status: "idle" | "working" | "attention";
  tags: string[];
}

export interface Provider {
  id: Exclude<ProviderId, "auto">;
  name: string;
  shortName: string;
  model: string;
  status: "connected" | "ready" | "login-required" | "unavailable" | "update";
  version: string;
  color: string;
  capabilities: string[];
  source: string;
  description: string;
}

export interface Message {
  id: string;
  conversationId: string;
  role: "user" | "assistant" | "system";
  author: string;
  content: string;
  createdAt: string;
  provider?: string;
  model?: string;
  duration?: string;
}

export interface TaskRun {
  id: string;
  title: string;
  description: string;
  status: RunStatus;
  provider: string;
  model?: string;
  duration?: string;
  parentId?: string;
  progress?: number;
  events: RunEvent[];
}

export interface RunEvent {
  id: string;
  kind: "read" | "search" | "tool" | "note" | "write" | "test" | "error";
  label: string;
  detail?: string;
  timestamp: string;
}

export interface FileChange {
  path: string;
  additions: number;
  deletions: number;
  status: "modified" | "added" | "deleted";
}

export interface UsageDatum {
  provider: string;
  color: string;
  percent: number;
  requests: number;
  tokens?: string;
  cost?: string;
}

export interface HardwareProfile {
  device: string;
  cpu: string;
  memory: string;
  gpu: string;
  freeDisk: string;
  tier: number;
}

export interface ComposerDraft {
  text: string;
  provider: ProviderId;
  routingMode: RoutingMode;
  attachments: string[];
}

export interface SearchResult {
  id: string;
  type: "conversation" | "message" | "file" | "command";
  title: string;
  excerpt: string;
  project?: string;
  conversationId?: string;
  command?: string;
}

export interface HarnessSnapshot {
  projects: Project[];
  groups: ChatGroup[];
  conversations: Conversation[];
  providers: Provider[];
  messages: Message[];
  tasks: TaskRun[];
  files: FileChange[];
  usage: UsageDatum[];
  hardware: HardwareProfile;
}

export type ComponentKind = "APP" | "PROVIDER" | "RUNTIME" | "MODEL";

export interface ManagedComponent {
  id: string;
  name: string;
  kind: ComponentKind;
  version: string;
  latestVersion: string;
  source: string;
  status: "latest" | "update" | "external";
  updatePolicy: "Automatic" | "Notify only" | "Manual";
}
