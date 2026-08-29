import { invoke } from "@tauri-apps/api/core";

import { mockSnapshot } from "../data/mockData";
import type { ComposerDraft, HarnessSnapshot, SearchResult } from "../types";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

export interface HarnessBridge {
  getSnapshot(): Promise<HarnessSnapshot>;
  createProject(name: string, workspace: string): Promise<{ id: string }>;
  createConversation(projectId: string, groupId: string): Promise<{ id: string }>;
  sendMessage(
    conversationId: string,
    draft: ComposerDraft,
  ): Promise<{ accepted: boolean; dispatched: boolean }>;
  cancelRun(runId: string): Promise<void>;
  search(query: string): Promise<SearchResult[]>;
  chooseWorkspace(): Promise<string | null>;
}

const delay = (milliseconds: number) =>
  new Promise<void>((resolve) => window.setTimeout(resolve, milliseconds));

let browserSnapshot = structuredClone(mockSnapshot);

function localSearch(query: string): SearchResult[] {
  const normalized = query.trim().toLocaleLowerCase();
  if (!normalized) return [];

  const projectName = (projectId: string) =>
    browserSnapshot.projects.find((project) => project.id === projectId)?.name ?? "Unknown project";

  const conversations: SearchResult[] = browserSnapshot.conversations
    .filter((conversation) =>
      `${conversation.title} ${conversation.preview} ${conversation.tags.join(" ")}`
        .toLocaleLowerCase()
        .includes(normalized),
    )
    .map((conversation) => ({
      id: `conversation-${conversation.id}`,
      type: "conversation",
      title: conversation.title,
      excerpt: conversation.preview,
      project: projectName(conversation.projectId),
      conversationId: conversation.id,
    }));

  const messages: SearchResult[] = browserSnapshot.messages
    .filter((message) => message.content.toLocaleLowerCase().includes(normalized))
    .map((message) => ({
      id: `message-${message.id}`,
      type: "message",
      title: message.author,
      excerpt: message.content,
      conversationId: message.conversationId,
    }));

  const files: SearchResult[] = browserSnapshot.files
    .filter((file) => file.path.toLocaleLowerCase().includes(normalized))
    .map((file) => ({
      id: `file-${file.path}`,
      type: "file",
      title: file.path.split("/").at(-1) ?? file.path,
      excerpt: file.path,
    }));

  return [...conversations, ...messages, ...files].slice(0, 12);
}

const browserBridge: HarnessBridge = {
  async getSnapshot() {
    await delay(180);
    return structuredClone(browserSnapshot);
  },
  async createProject(name, workspace) {
    await delay(120);
    const id = `project-${Date.now()}`;
    browserSnapshot.projects.push({
      id,
      name,
      description: "",
      color: "#5b7cfa",
      workspace,
      conversations: 0,
      updatedAt: "Now",
    });
    browserSnapshot.groups.push({
      id: `group-${Date.now()}`,
      projectId: id,
      name: "General",
    });
    return { id };
  },
  async createConversation(projectId, groupId) {
    await delay(100);
    const id = `conversation-${Date.now()}`;
    browserSnapshot.conversations.unshift({
      id,
      projectId,
      groupId,
      title: "Untitled conversation",
      preview: "Ready when you are",
      updatedAt: "Now",
      status: "idle",
      tags: [],
    });
    const project = browserSnapshot.projects.find((item) => item.id === projectId);
    if (project) project.conversations += 1;
    return { id };
  },
  async sendMessage(conversationId, draft) {
    await delay(350);
    browserSnapshot.messages.push({
      id: `message-${Date.now()}`,
      conversationId,
      role: "user",
      author: "You",
      content: draft.text,
      createdAt: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }),
    });
    return { accepted: true, dispatched: true };
  },
  async cancelRun() {
    await delay(120);
  },
  async search(query) {
    await delay(80);
    return localSearch(query);
  },
  async chooseWorkspace() {
    await delay(150);
    return "/Users/you/Projects/harness-desktop";
  },
};

const tauriBridge: HarnessBridge = {
  getSnapshot: () => invoke<HarnessSnapshot>("get_harness_snapshot"),
  async createProject(name, workspace) {
    const project = await invoke<{ id: string }>("create_project", {
      input: {
        name,
        description: "",
        workspace_path: workspace || null,
        instructions: "",
        tags: [],
        routing_mode: "balanced",
      },
    });
    await invoke("create_chat_group", {
      input: { project_id: project.id, name: "General" },
    });
    return project;
  },
  createConversation: (projectId, groupId) =>
    invoke<{ id: string }>("create_conversation", {
      input: {
        project_id: projectId,
        group_id: groupId,
        title: "Untitled conversation",
        tags: [],
        provider_policy: null,
        workspace_path: null,
      },
    }),
  sendMessage: (conversationId, draft) =>
    invoke<{ accepted: boolean; dispatched: boolean }>("send_message", { conversationId, draft }),
  cancelRun: (runId) => invoke<void>("cancel_agent_run", { runId }),
  search: (query) => invoke<SearchResult[]>("search_harness", { query }),
  chooseWorkspace: () => invoke<string | null>("choose_workspace"),
};

// Demo data is only used by the ordinary browser/Vite preview. A native error must surface to
// the user instead of being silently presented as a successful agent run.
export const bridge: HarnessBridge = window.__TAURI_INTERNALS__ ? tauriBridge : browserBridge;
