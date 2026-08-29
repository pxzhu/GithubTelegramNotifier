import { useEffect, useRef, useState } from "react";

import { Icon } from "./Icon";
import type {
  ComposerDraft,
  Conversation,
  Message,
  Project,
  ProviderId,
  RoutingMode,
  TaskRun,
} from "../types";

interface WorkspaceProps {
  project?: Project;
  conversation?: Conversation;
  messages: Message[];
  tasks: TaskRun[];
  inspectorOpen: boolean;
  isSending: boolean;
  onToggleInspector: () => void;
  onOpenActivity: () => void;
  onCreateProject: () => void;
  onNewConversation: () => void;
  onSend: (draft: ComposerDraft) => void;
  onCancelRun: (runId: string) => void;
}

const providerOptions: Array<{ value: ProviderId; label: string }> = [
  { value: "auto", label: "Auto" },
  { value: "claude", label: "Claude Code" },
  { value: "m365", label: "Microsoft 365" },
  { value: "local", label: "Local AI" },
  { value: "codex", label: "Codex CLI" },
];

const routingOptions: Array<{ value: RoutingMode; label: string }> = [
  { value: "balanced", label: "Balanced" },
  { value: "quality", label: "Quality First" },
  { value: "claude-saver", label: "Claude Saver" },
  { value: "local-first", label: "Local First" },
];

function RunStatusIcon({ status }: { status: TaskRun["status"] }) {
  if (status === "completed") return <span className="run-status completed"><Icon name="check" size={12} /></span>;
  if (status === "running") return <span className="run-status running"><span /></span>;
  if (status === "failed") return <span className="run-status failed"><Icon name="close" size={12} /></span>;
  return <span className="run-status queued" />;
}

function MiniTaskGraph({ tasks, onOpenActivity }: { tasks: TaskRun[]; onOpenActivity: () => void }) {
  const completed = tasks.filter((task) => task.status === "completed").length;
  const running = tasks.find((task) => task.status === "running");

  return (
    <section className="inline-task-graph" aria-label="Agent task progress">
      <div className="task-graph-header">
        <div>
          <span className="eyebrow"><Icon name="sparkles" size={13} /> Agent run</span>
          <h3>Working on {tasks.length} tasks</h3>
        </div>
        <button className="text-button" onClick={onOpenActivity}>Open activity <Icon name="arrow" size={13} /></button>
      </div>
      <div className="task-progress-line">
        <span style={{ width: `${(completed / tasks.length) * 100}%` }} />
      </div>
      <div className="task-node-row">
        {tasks.map((task, index) => (
          <div className={`task-node ${task.status}`} key={task.id}>
            <div className="node-track">
              <RunStatusIcon status={task.status} />
              {index < tasks.length - 1 && <span className="node-connector" />}
            </div>
            <span className="node-label">{task.title}</span>
            <small>{task.provider}</small>
          </div>
        ))}
      </div>
      {running && (
        <div className="current-run-row">
          <span className="agent-orb codex">CX</span>
          <span><strong>{running.title}</strong><small>{running.events.at(-1)?.label}</small></span>
          <span className="run-progress-number">{running.progress ?? 0}%</span>
        </div>
      )}
    </section>
  );
}

function Composer({ onSend, isSending }: { onSend: (draft: ComposerDraft) => void; isSending: boolean }) {
  const [text, setText] = useState("");
  const [provider, setProvider] = useState<ProviderId>("auto");
  const [routingMode, setRoutingMode] = useState<RoutingMode>("claude-saver");
  const [attachments, setAttachments] = useState<string[]>([]);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    const textarea = textareaRef.current;
    if (!textarea) return;
    textarea.style.height = "auto";
    textarea.style.height = `${Math.min(textarea.scrollHeight, 150)}px`;
  }, [text]);

  const submit = () => {
    if (!text.trim() || isSending) return;
    onSend({ text: text.trim(), provider, routingMode, attachments });
    setText("");
    setAttachments([]);
  };

  return (
    <div className="composer-wrap">
      {attachments.length > 0 && (
        <div className="attachment-row">
          {attachments.map((attachment) => (
            <span className="attachment-chip" key={attachment}><Icon name="file" size={13} /> {attachment}<button onClick={() => setAttachments([])} aria-label={`Remove ${attachment}`}><Icon name="close" size={11} /></button></span>
          ))}
        </div>
      )}
      <div className="composer">
        <textarea
          ref={textareaRef}
          aria-label="Message Harness"
          placeholder="Ask Harness to research, build, or fix something…"
          value={text}
          onChange={(event) => setText(event.target.value)}
          onKeyDown={(event) => {
            if (event.key === "Enter" && (event.metaKey || event.ctrlKey)) {
              event.preventDefault();
              submit();
            }
          }}
          rows={1}
        />
        <div className="composer-toolbar">
          <div className="composer-tools">
            <button
              className="icon-button"
              aria-label="Attach a file"
              onClick={() => setAttachments(["incident-notes.md"])}
            ><Icon name="attach" size={17} /></button>
            <span className="toolbar-divider" />
            <label className="select-control provider-select">
              <span className={`provider-mini provider-${provider}`}><Icon name={provider === "auto" ? "sparkles" : provider === "local" ? "bolt" : provider === "m365" ? "grid" : "terminal"} size={13} /></span>
              <select value={provider} onChange={(event) => setProvider(event.target.value as ProviderId)} aria-label="Provider">
                {providerOptions.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
              </select>
              <Icon name="chevron" size={12} />
            </label>
            {provider === "auto" && (
              <label className="select-control routing-select">
                <span className="saving-leaf">◒</span>
                <select value={routingMode} onChange={(event) => setRoutingMode(event.target.value as RoutingMode)} aria-label="Routing mode">
                  {routingOptions.map((option) => <option key={option.value} value={option.value}>{option.label}</option>)}
                </select>
                <Icon name="chevron" size={12} />
              </label>
            )}
          </div>
          <button className={`send-button ${isSending ? "is-sending" : ""}`} onClick={submit} disabled={!text.trim() || isSending} aria-label="Send message">
            {isSending ? <span className="tiny-spinner" /> : <Icon name="send" size={15} />}
          </button>
        </div>
      </div>
      <div className="composer-footnote">
        <span><Icon name="info" size={12} /> Harness may request approval before modifying files or running commands.</span>
        <span><kbd>⌘</kbd><kbd>↵</kbd> Send</span>
      </div>
    </div>
  );
}

export function Workspace({
  project,
  conversation,
  messages,
  tasks,
  inspectorOpen,
  isSending,
  onToggleInspector,
  onOpenActivity,
  onCreateProject,
  onNewConversation,
  onSend,
  onCancelRun,
}: WorkspaceProps) {
  const runningTask = tasks.find((task) => task.status === "running");

  if (!conversation) {
    return (
      <main className="workspace empty-workspace">
        <div className="empty-workspace-art"><span /><span /><span /></div>
        <h2>{project ? "Start something ambitious" : "Create your first project"}</h2>
        <p>{project ? "Create a conversation and Harness will choose the right agents for the work." : "Projects keep conversations, workspaces, policies, and usage isolated on this device."}</p>
        <button className="primary-button empty-state-action" onClick={project ? onNewConversation : onCreateProject}><Icon name="plus" size={14} /> {project ? "New conversation" : "Create project"}</button>
      </main>
    );
  }

  return (
    <main className="workspace">
      <header className="workspace-header">
        <div className="workspace-title">
          <div className="title-row"><h2>{conversation.title}</h2><span className={`status-badge ${conversation.status}`}>{conversation.status === "working" ? "Working" : conversation.status === "attention" ? "Attention" : "Ready"}</span></div>
          <div className="workspace-breadcrumb">
            <span>{project?.name}</span><span>›</span><span>{project?.branch ?? "Local workspace"}</span>
          </div>
        </div>
        <div className="workspace-actions">
          {runningTask && <button className="stop-button" onClick={() => onCancelRun(runningTask.id)}><Icon name="stop" size={13} /> Stop</button>}
          <button className="icon-button" aria-label="Conversation options"><Icon name="more" /></button>
          <button className={`inspector-button ${inspectorOpen ? "active" : ""}`} onClick={onToggleInspector} aria-label="Toggle inspector"><Icon name="sidebar" size={17} /><span>Inspect</span></button>
        </div>
      </header>

      <div className="message-scroll">
        <div className="message-column">
          <div className="conversation-date"><span>Today</span></div>
          {messages.map((message, index) => (
            <article className={`message ${message.role}`} key={message.id}>
              <div className={`message-avatar ${message.role}`}>
                {message.role === "assistant" ? <span className="brand-mark small"><span /><span /><span /></span> : "P"}
              </div>
              <div className="message-body">
                <header><strong>{message.author}</strong><time>{message.createdAt}</time></header>
                <p>{message.content}</p>
                {message.role === "assistant" && (
                  <div className="message-meta">
                    <span><Icon name="sparkles" size={12} /> {message.provider}</span>
                    {message.duration && <span>{message.duration}</span>}
                    <button aria-label="Copy response">Copy</button>
                  </div>
                )}
                {index === messages.length - 1 && tasks.length > 0 && <MiniTaskGraph tasks={tasks} onOpenActivity={onOpenActivity} />}
              </div>
            </article>
          ))}
        </div>
      </div>

      <Composer onSend={onSend} isSending={isSending} />
    </main>
  );
}
