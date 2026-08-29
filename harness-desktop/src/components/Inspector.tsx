import { useState } from "react";

import { Icon } from "./Icon";
import type { FileChange, InspectorTab, Project, TaskRun, UsageDatum } from "../types";

interface InspectorProps {
  activeTab: InspectorTab;
  tasks: TaskRun[];
  files: FileChange[];
  usage: UsageDatum[];
  project?: Project;
  onTabChange: (tab: InspectorTab) => void;
  onClose: () => void;
}

const tabs: Array<{ id: InspectorTab; label: string }> = [
  { id: "activity", label: "Activity" },
  { id: "usage", label: "Usage" },
  { id: "context", label: "Context" },
  { id: "files", label: "Files" },
];

const eventIcon = (kind: TaskRun["events"][number]["kind"]) => {
  if (kind === "search") return "search";
  if (kind === "write") return "code";
  if (kind === "test") return "check";
  if (kind === "read") return "file";
  if (kind === "error") return "warning";
  return "terminal";
};

function ActivityPanel({ tasks }: { tasks: TaskRun[] }) {
  const [expanded, setExpanded] = useState<Set<string>>(new Set(["test", "analysis"]));
  const completed = tasks.filter((task) => task.status === "completed").length;

  if (tasks.length === 0) {
    return (
      <div className="inspector-panel activity-panel inspector-empty-state">
        <span><Icon name="activity" size={18} /></span>
        <strong>No agent run yet</strong>
        <p>Task and tool events will appear here after a configured provider starts work.</p>
      </div>
    );
  }

  const toggle = (taskId: string) => {
    setExpanded((current) => {
      const next = new Set(current);
      if (next.has(taskId)) next.delete(taskId);
      else next.add(taskId);
      return next;
    });
  };

  return (
    <div className="inspector-panel activity-panel">
      <div className="run-overview">
        <div className="run-overview-top"><div><span className="live-indicator"><span /> Run in progress</span><strong>{completed} of {tasks.length} tasks complete</strong></div><span className="elapsed">2m 18s</span></div>
        <div className="overview-progress"><span style={{ width: `${(completed / tasks.length) * 100}%` }} /></div>
        <div className="routing-summary"><span><Icon name="sparkles" size={13} /> Claude Saver</span><span>3 providers</span></div>
      </div>

      <div className="task-timeline">
        {tasks.map((task, index) => {
          const isExpanded = expanded.has(task.id);
          return (
            <div className={`timeline-task ${task.status}`} key={task.id}>
              <div className="timeline-rail"><span className="timeline-dot">{task.status === "completed" ? <Icon name="check" size={11} /> : task.status === "running" ? <span className="pulse-dot" /> : null}</span>{index < tasks.length - 1 && <span className="timeline-line" />}</div>
              <div className="timeline-content">
                <button className="timeline-heading" onClick={() => toggle(task.id)} aria-expanded={isExpanded}>
                  <span><strong>{task.title}</strong><small>{task.description}</small></span>
                  <Icon className={isExpanded ? "rotated" : ""} name="arrow" size={13} />
                </button>
                <div className="task-provider-row"><span className={`agent-orb ${task.provider.toLocaleLowerCase().includes("claude") ? "claude" : task.provider.toLocaleLowerCase().includes("microsoft") ? "m365" : task.provider.toLocaleLowerCase().includes("codex") ? "codex" : "local"}`}>{task.provider === "Claude Code" ? "CL" : task.provider === "Microsoft 365" ? "M" : task.provider === "Codex CLI" ? "CX" : "L"}</span><span>{task.provider}</span>{task.duration && <time>{task.duration}</time>}</div>
                {task.status === "running" && <div className="task-inline-progress"><span style={{ width: `${task.progress}%` }} /></div>}
                {isExpanded && (
                  <div className="event-list">
                    {task.events.map((event) => (
                      <div className="event-row" key={event.id}>
                        <span className="event-icon"><Icon name={eventIcon(event.kind)} size={12} /></span>
                        <span><strong>{event.label}</strong>{event.detail && <small>{event.detail}</small>}</span>
                        <time>{event.timestamp}</time>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}

function UsagePanel({ usage }: { usage: UsageDatum[] }) {
  if (usage.length === 0) {
    return (
      <div className="inspector-panel usage-inspector inspector-empty-state">
        <span><Icon name="activity" size={18} /></span>
        <strong>No usage recorded</strong>
        <p>Known provider usage is stored locally. Values a provider does not expose remain unavailable.</p>
      </div>
    );
  }

  const knownTokens = usage.find((datum) => datum.tokens)?.tokens ?? "Unavailable";
  return (
    <div className="inspector-panel usage-inspector">
      <div className="inspector-metric"><span>This run</span><strong>{knownTokens}</strong><small>known provider usage</small></div>
      <div className="stacked-usage-bar" aria-label="Usage by provider">
        {usage.map((datum) => <span key={datum.provider} style={{ width: `${datum.percent}%`, background: datum.color }} />)}
      </div>
      <div className="inspector-usage-list">
        {usage.map((datum) => (
          <div className="inspector-usage-row" key={datum.provider}>
            <span className="usage-swatch" style={{ background: datum.color }} />
            <span><strong>{datum.provider}</strong><small>{datum.requests} requests</small></span>
            <span>{datum.tokens ?? "Unavailable"}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

function ContextPanel({ project }: { project?: Project }) {
  return (
    <div className="inspector-panel context-panel">
      <div className="context-block"><span className="context-icon"><Icon name="folder" size={16} /></span><div><strong>Project workspace</strong><p>{project?.workspace}</p></div><span className="context-state">Included</span></div>
      <div className="context-block"><span className="context-icon blue"><Icon name="grid" size={16} /></span><div><strong>Microsoft 365</strong><p>Enterprise context is retrieved only when a task requires it.</p></div><span className="context-state">On demand</span></div>
      <div className="context-block"><span className="context-icon violet"><Icon name="message" size={16} /></span><div><strong>Conversation</strong><p>Relevant messages and summaries are selected per task.</p></div><span className="context-state">On demand</span></div>
      <section className="instruction-card"><header><strong>Project instructions</strong><button>Edit</button></header><p>No instructions are exposed in this conversation snapshot.</p></section>
      <button className="secondary-button full-width"><Icon name="plus" size={14} /> Add context</button>
    </div>
  );
}

function FilesPanel({ files }: { files: FileChange[] }) {
  const additions = files.reduce((sum, file) => sum + file.additions, 0);
  const deletions = files.reduce((sum, file) => sum + file.deletions, 0);
  return (
    <div className="inspector-panel files-panel">
      <div className="file-summary"><span>{files.length} changed files</span><span className="diff-add">+{additions}</span><span className="diff-delete">−{deletions}</span></div>
      <div className="file-change-list">
        {files.map((file) => (
          <button className="file-change" key={file.path}>
            <span className={`file-status ${file.status}`}>{file.status === "added" ? "A" : file.status === "deleted" ? "D" : "M"}</span>
            <span><strong>{file.path.split("/").at(-1)}</strong><small>{file.path.split("/").slice(0, -1).join("/")}/</small></span>
            <span className="file-diff"><em>+{file.additions}</em>{file.deletions > 0 && <i>−{file.deletions}</i>}</span>
          </button>
        ))}
      </div>
      <button className="primary-outline-button full-width"><Icon name="code" size={15} /> Review all changes</button>
      <div className="file-safety-note"><Icon name="info" size={14} /><span>Changes remain in your local workspace. Harness never commits without your approval.</span></div>
    </div>
  );
}

export function Inspector({ activeTab, tasks, files, usage, project, onTabChange, onClose }: InspectorProps) {
  return (
    <aside className="inspector" aria-label="Inspector">
      <header className="inspector-header">
        <div className="inspector-tabs" role="tablist" aria-label="Inspector views">
          {tabs.map((tab) => (
            <button className={activeTab === tab.id ? "active" : ""} key={tab.id} role="tab" aria-selected={activeTab === tab.id} onClick={() => onTabChange(tab.id)}>{tab.label}</button>
          ))}
        </div>
        <button className="icon-button" onClick={onClose} aria-label="Close inspector"><Icon name="close" size={16} /></button>
      </header>
      {activeTab === "activity" && <ActivityPanel tasks={tasks} />}
      {activeTab === "usage" && <UsagePanel usage={usage} />}
      {activeTab === "context" && <ContextPanel project={project} />}
      {activeTab === "files" && <FilesPanel files={files} />}
    </aside>
  );
}
