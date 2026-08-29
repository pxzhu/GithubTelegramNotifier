import { useEffect, useMemo, useRef, useState } from "react";

import { Icon, type IconName } from "./Icon";
import type { Project, Screen } from "../types";

interface CommandPaletteProps {
  open: boolean;
  projects: Project[];
  onClose: () => void;
  onNavigate: (screen: Screen) => void;
  onProject: (projectId: string) => void;
  onNewChat: () => void;
}

interface CommandItem {
  id: string;
  label: string;
  detail: string;
  icon: IconName;
  shortcut?: string;
  action: () => void;
}

export function CommandPalette({ open, projects, onClose, onNavigate, onProject, onNewChat }: CommandPaletteProps) {
  const [query, setQuery] = useState("");
  const [selected, setSelected] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const commands = useMemo<CommandItem[]>(() => [
    { id: "new", label: "New conversation", detail: "Start in the current project", icon: "plus", shortcut: "⌘ N", action: onNewChat },
    { id: "search", label: "Search everything", detail: "Messages, files, results and tags", icon: "search", shortcut: "⌘ ⇧ F", action: () => onNavigate("search") },
    { id: "usage", label: "Open usage dashboard", detail: "Providers, projects and savings", icon: "activity", action: () => onNavigate("usage") },
    { id: "providers", label: "Manage providers", detail: "Connections, models and permissions", icon: "layers", action: () => onNavigate("settings") },
    { id: "settings", label: "Open settings", detail: "Preferences and security", icon: "gear", shortcut: "⌘ ,", action: () => onNavigate("settings") },
    ...projects.map((project) => ({ id: `project-${project.id}`, label: project.name, detail: project.workspace, icon: "folder" as const, action: () => onProject(project.id) })),
  ], [onNavigate, onNewChat, onProject, projects]);

  const visibleCommands = useMemo(() => {
    const normalized = query.toLocaleLowerCase().trim();
    return normalized ? commands.filter((command) => `${command.label} ${command.detail}`.toLocaleLowerCase().includes(normalized)) : commands;
  }, [commands, query]);

  useEffect(() => {
    if (open) {
      setQuery("");
      setSelected(0);
      window.setTimeout(() => inputRef.current?.focus(), 0);
    }
  }, [open]);

  useEffect(() => {
    if (selected >= visibleCommands.length) setSelected(Math.max(visibleCommands.length - 1, 0));
  }, [selected, visibleCommands.length]);

  if (!open) return null;

  const run = (command: CommandItem) => {
    command.action();
    onClose();
  };

  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={onClose}>
      <section className="command-palette" role="dialog" aria-modal="true" aria-label="Command palette" onMouseDown={(event) => event.stopPropagation()}>
        <label className="command-search">
          <Icon name="search" size={20} />
          <input
            ref={inputRef}
            value={query}
            onChange={(event) => { setQuery(event.target.value); setSelected(0); }}
            onKeyDown={(event) => {
              if (event.key === "Escape") onClose();
              if (event.key === "ArrowDown") { event.preventDefault(); setSelected((value) => Math.min(value + 1, visibleCommands.length - 1)); }
              if (event.key === "ArrowUp") { event.preventDefault(); setSelected((value) => Math.max(value - 1, 0)); }
              if (event.key === "Enter" && visibleCommands[selected]) run(visibleCommands[selected]);
            }}
            placeholder="Search projects and commands…"
            aria-label="Search commands"
          />
          <kbd>esc</kbd>
        </label>
        <div className="command-results" role="listbox">
          <span className="command-section-label">{query ? "Results" : "Suggested"}</span>
          {visibleCommands.map((command, index) => (
            <button className={`command-result ${selected === index ? "selected" : ""}`} key={command.id} role="option" aria-selected={selected === index} onMouseEnter={() => setSelected(index)} onClick={() => run(command)}>
              <span className="command-icon"><Icon name={command.icon} size={17} /></span>
              <span><strong>{command.label}</strong><small>{command.detail}</small></span>
              {command.shortcut && <kbd>{command.shortcut}</kbd>}
            </button>
          ))}
          {visibleCommands.length === 0 && <div className="command-empty"><span>No matching command</span><small>Try a project name or action.</small></div>}
        </div>
        <footer className="command-footer"><span><kbd>↑</kbd><kbd>↓</kbd> Navigate</span><span><kbd>↵</kbd> Open</span><span className="command-local"><span /> Local index</span></footer>
      </section>
    </div>
  );
}
