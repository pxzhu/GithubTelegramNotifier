import { useCallback, useEffect, useMemo, useState } from "react";

import { CommandPalette } from "./components/CommandPalette";
import { ConversationList } from "./components/ConversationList";
import { CreateProjectDialog } from "./components/CreateProjectDialog";
import { Inspector } from "./components/Inspector";
import { ProjectSidebar } from "./components/ProjectSidebar";
import { SearchScreen } from "./components/SearchScreen";
import { SettingsScreen } from "./components/SettingsScreen";
import { UsageScreen } from "./components/UsageScreen";
import { Workspace } from "./components/Workspace";
import { Icon } from "./components/Icon";
import { bridge } from "./lib/bridge";
import type { ComposerDraft, HarnessSnapshot, InspectorTab, Message, Screen, SearchResult } from "./types";

function AppLoading() {
  return <div className="app-loading"><div className="loading-brand"><span className="brand-mark large"><span /><span /><span /></span><strong>Harness</strong></div><div className="loading-track"><span /></div><p>Opening your local workspace…</p></div>;
}

function errorMessage(error: unknown, fallback: string) {
  if (error instanceof Error && error.message) return error.message;
  if (error && typeof error === "object" && "message" in error && typeof error.message === "string") return error.message;
  if (typeof error === "string" && error.trim()) return error;
  return fallback;
}

export default function App() {
  const [snapshot, setSnapshot] = useState<HarnessSnapshot | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [screen, setScreen] = useState<Screen>("workspace");
  const [activeProjectId, setActiveProjectId] = useState("harness");
  const [activeConversationId, setActiveConversationId] = useState("auth-bug");
  const [projectSidebarCollapsed, setProjectSidebarCollapsed] = useState(false);
  const [inspectorOpen, setInspectorOpen] = useState(true);
  const [inspectorTab, setInspectorTab] = useState<InspectorTab>("activity");
  const [commandOpen, setCommandOpen] = useState(false);
  const [isSending, setIsSending] = useState(false);
  const [projectDialogOpen, setProjectDialogOpen] = useState(false);
  const [projectCreating, setProjectCreating] = useState(false);
  const [toast, setToast] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoadError(null);
    try {
      const next = await bridge.getSnapshot();
      setSnapshot(next);
      return next;
    } catch (error: unknown) {
      setLoadError(errorMessage(error, "Harness Core did not respond."));
      return undefined;
    }
  }, []);

  useEffect(() => { void load(); }, [load]);

  useEffect(() => {
    if (!snapshot) return;
    const project = snapshot.projects.find((item) => item.id === activeProjectId && !item.archived)
      ?? snapshot.projects.find((item) => !item.archived);
    const nextProjectId = project?.id ?? "";
    if (nextProjectId !== activeProjectId) setActiveProjectId(nextProjectId);
    const conversation = snapshot.conversations.find((item) => item.id === activeConversationId && item.projectId === nextProjectId && !item.archived)
      ?? snapshot.conversations.find((item) => item.projectId === nextProjectId && !item.archived);
    const nextConversationId = conversation?.id ?? "";
    if (nextConversationId !== activeConversationId) setActiveConversationId(nextConversationId);
  }, [activeConversationId, activeProjectId, snapshot]);

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const modifier = event.metaKey || event.ctrlKey;
      if (modifier && event.key.toLocaleLowerCase() === "k") { event.preventDefault(); setCommandOpen((open) => !open); }
      if (modifier && event.shiftKey && event.key.toLocaleLowerCase() === "f") { event.preventDefault(); setScreen("search"); }
      if (modifier && event.key === ",") { event.preventDefault(); setScreen("settings"); }
      if (modifier && event.key.toLocaleLowerCase() === "n") { event.preventDefault(); createConversation(); }
      if (event.key === "Escape") setCommandOpen(false);
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  });

  useEffect(() => {
    if (!toast) return;
    const timer = window.setTimeout(() => setToast(null), 2600);
    return () => window.clearTimeout(timer);
  }, [toast]);

  const activeProject = snapshot?.projects.find((project) => project.id === activeProjectId);
  const projectGroups = snapshot?.groups.filter((group) => group.projectId === activeProjectId) ?? [];
  const projectConversations = snapshot?.conversations.filter((conversation) => conversation.projectId === activeProjectId && !conversation.archived) ?? [];
  const activeConversation = snapshot?.conversations.find((conversation) => conversation.id === activeConversationId);
  const conversationMessages = snapshot?.messages.filter((message) => message.conversationId === activeConversationId) ?? [];

  const navigate = useCallback((nextScreen: Screen) => setScreen(nextScreen), []);

  const selectProject = useCallback((projectId: string) => {
    if (!snapshot) return;
    setActiveProjectId(projectId);
    const nextConversation = snapshot.conversations.find((conversation) => conversation.projectId === projectId && !conversation.archived);
    if (nextConversation) setActiveConversationId(nextConversation.id);
    setScreen("workspace");
  }, [snapshot]);

  const selectConversation = useCallback((conversationId: string) => {
    if (!snapshot) return;
    const conversation = snapshot.conversations.find((item) => item.id === conversationId);
    if (conversation) setActiveProjectId(conversation.projectId);
    setActiveConversationId(conversationId);
    setScreen("workspace");
  }, [snapshot]);

  async function createConversation() {
    if (!snapshot) return;
    if (!activeProjectId) {
      setProjectDialogOpen(true);
      return;
    }
    const group = snapshot.groups.find((item) => item.projectId === activeProjectId);
    if (!group) {
      setToast("This project does not have a chat group yet");
      return;
    }
    try {
      const conversation = await bridge.createConversation(activeProjectId, group.id);
      await load();
      setActiveConversationId(conversation.id);
      setScreen("workspace");
      setToast("New conversation created");
    } catch (error: unknown) {
      setToast(errorMessage(error, "The conversation could not be created"));
    }
  }

  const createProject = useCallback(async (name: string, workspace: string) => {
    setProjectCreating(true);
    try {
      const project = await bridge.createProject(name, workspace);
      await load();
      setActiveProjectId(project.id);
      setActiveConversationId("");
      setScreen("workspace");
      setProjectDialogOpen(false);
      setToast("Project created locally");
    } catch (error: unknown) {
      setToast(errorMessage(error, "The project could not be created"));
    } finally {
      setProjectCreating(false);
    }
  }, [load]);

  const toggleFavorite = useCallback((conversationId: string) => {
    setSnapshot((current) => current ? ({ ...current, conversations: current.conversations.map((conversation) => conversation.id === conversationId ? { ...conversation, favorite: !conversation.favorite } : conversation) }) : current);
  }, []);

  const sendMessage = useCallback((draft: ComposerDraft) => {
    if (!snapshot) return;
    const message: Message = { id: `message-${Date.now()}`, conversationId: activeConversationId, role: "user", author: "You", content: draft.text, createdAt: new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }) };
    setSnapshot((current) => current ? ({ ...current, messages: [...current.messages, message], conversations: current.conversations.map((conversation) => conversation.id === activeConversationId ? { ...conversation, title: conversation.title === "Untitled conversation" ? draft.text.slice(0, 42) : conversation.title, preview: draft.text, updatedAt: "Now", status: "working" } : conversation) }) : current);
    setIsSending(true);
    bridge.sendMessage(activeConversationId, draft).then((result) => {
      if (!result.accepted) throw new Error("The message was not accepted");
      if (result.dispatched) {
        setToast(`${draft.provider === "auto" ? "Auto router" : draft.provider} accepted the task`);
      } else {
        setSnapshot((current) => current ? ({ ...current, conversations: current.conversations.map((conversation) => conversation.id === activeConversationId ? { ...conversation, status: "idle" } : conversation) }) : current);
        setToast("Message saved — provider execution is not configured yet");
      }
    }).catch((error: unknown) => {
      setSnapshot((current) => current ? ({ ...current, messages: current.messages.filter((item) => item.id !== message.id), conversations: current.conversations.map((conversation) => conversation.id === activeConversationId ? { ...conversation, status: "attention" } : conversation) }) : current);
      setToast(errorMessage(error, "The task could not be started"));
    }).finally(() => setIsSending(false));
  }, [activeConversationId, snapshot]);

  const cancelRun = useCallback((runId: string) => {
    bridge.cancelRun(runId).then(() => {
      setSnapshot((current) => current ? ({ ...current, tasks: current.tasks.map((task) => task.id === runId ? { ...task, status: "failed", description: "Cancelled by user" } : task) }) : current);
      setToast("Agent run stopped");
    });
  }, []);

  const search = useCallback((query: string): Promise<SearchResult[]> => bridge.search(query), []);

  const shellClass = useMemo(() => ["app-shell", projectSidebarCollapsed ? "sidebar-collapsed" : "", screen !== "workspace" ? "utility-screen" : "", inspectorOpen && screen === "workspace" ? "inspector-visible" : ""].filter(Boolean).join(" "), [inspectorOpen, projectSidebarCollapsed, screen]);

  if (loadError) return <div className="fatal-state"><span className="fatal-icon"><Icon name="warning" /></span><h1>Couldn’t open Harness</h1><p>{loadError}</p><button className="primary-button" onClick={load}><Icon name="refresh" size={14} /> Try again</button></div>;
  if (!snapshot) return <AppLoading />;

  return (
    <div className={shellClass}>
      <ProjectSidebar projects={snapshot.projects} activeProjectId={activeProjectId} screen={screen} collapsed={projectSidebarCollapsed} onSelectProject={selectProject} onNavigate={navigate} onToggle={() => setProjectSidebarCollapsed((value) => !value)} onNewProject={() => setProjectDialogOpen(true)} onOpenCommand={() => setCommandOpen(true)} />

      {screen === "workspace" ? <>
        <ConversationList project={activeProject} groups={projectGroups} conversations={projectConversations} activeConversationId={activeConversationId} onSelectConversation={selectConversation} onNewConversation={createConversation} onToggleFavorite={toggleFavorite} onOpenSearch={() => setScreen("search")} />
        <Workspace project={activeProject} conversation={activeConversation} messages={conversationMessages} tasks={snapshot.tasks} inspectorOpen={inspectorOpen} isSending={isSending} onToggleInspector={() => setInspectorOpen((value) => !value)} onOpenActivity={() => { setInspectorTab("activity"); setInspectorOpen(true); }} onCreateProject={() => setProjectDialogOpen(true)} onNewConversation={() => { void createConversation(); }} onSend={sendMessage} onCancelRun={cancelRun} />
        {inspectorOpen && <Inspector activeTab={inspectorTab} tasks={snapshot.tasks} files={snapshot.files} usage={snapshot.usage} project={activeProject} onTabChange={setInspectorTab} onClose={() => setInspectorOpen(false)} />}
      </> : screen === "search" ? <SearchScreen onSearch={search} onOpenConversation={selectConversation} /> : screen === "usage" ? <UsageScreen usage={snapshot.usage} /> : <SettingsScreen providers={snapshot.providers} hardware={snapshot.hardware} />}

      <CommandPalette open={commandOpen} projects={snapshot.projects} onClose={() => setCommandOpen(false)} onNavigate={navigate} onProject={selectProject} onNewChat={createConversation} />
      <CreateProjectDialog open={projectDialogOpen} busy={projectCreating} onClose={() => setProjectDialogOpen(false)} onChooseWorkspace={() => bridge.chooseWorkspace()} onCreate={createProject} />
      {toast && <div className="toast" role="status"><Icon name="check" size={14} />{toast}</div>}
    </div>
  );
}
