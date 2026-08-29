import { Icon } from "./Icon";
import type { Project, Screen } from "../types";

interface ProjectSidebarProps {
  projects: Project[];
  activeProjectId: string;
  screen: Screen;
  collapsed: boolean;
  onSelectProject: (projectId: string) => void;
  onNavigate: (screen: Screen) => void;
  onToggle: () => void;
  onNewProject: () => void;
  onOpenCommand: () => void;
}

export function ProjectSidebar({
  projects,
  activeProjectId,
  screen,
  collapsed,
  onSelectProject,
  onNavigate,
  onToggle,
  onNewProject,
  onOpenCommand,
}: ProjectSidebarProps) {
  return (
    <aside className={`project-sidebar ${collapsed ? "is-collapsed" : ""}`} aria-label="Projects">
      <div className="window-drag-region">
        <div className="traffic-lights" aria-hidden="true">
          <span className="traffic-light close" />
          <span className="traffic-light minimize" />
          <span className="traffic-light maximize" />
        </div>
        <button className="icon-button sidebar-toggle" onClick={onToggle} aria-label="Toggle project sidebar">
          <Icon name="sidebar" size={17} />
        </button>
      </div>

      <div className="brand-row">
        <div className="brand-mark" aria-hidden="true"><span /><span /><span /></div>
        {!collapsed && <span className="brand-name">Harness</span>}
      </div>

      <nav className="primary-nav" aria-label="Primary">
        <button className="nav-item command-trigger" onClick={onOpenCommand} title="Command palette">
          <Icon name="command" />
          {!collapsed && <><span>Quick open</span><kbd>⌘ K</kbd></>}
        </button>
        <button className={`nav-item ${screen === "search" ? "active" : ""}`} onClick={() => onNavigate("search")}>
          <Icon name="search" />
          {!collapsed && <span>Search</span>}
        </button>
        <button className={`nav-item ${screen === "usage" ? "active" : ""}`} onClick={() => onNavigate("usage")}>
          <Icon name="activity" />
          {!collapsed && <span>Usage</span>}
        </button>
      </nav>

      <div className="sidebar-section-heading">
        {!collapsed && <span>Projects</span>}
        <button className="icon-button subtle" onClick={onNewProject} aria-label="Create project">
          <Icon name="plus" size={15} />
        </button>
      </div>

      <div className="project-list">
        {projects.filter((project) => !project.archived).map((project) => (
          <button
            className={`project-item ${screen === "workspace" && project.id === activeProjectId ? "active" : ""}`}
            key={project.id}
            onClick={() => onSelectProject(project.id)}
            title={project.name}
          >
            <span className="project-avatar" style={{ "--project-color": project.color } as React.CSSProperties}>
              {project.name.slice(0, 1)}
            </span>
            {!collapsed && (
              <span className="project-meta">
                <span className="project-name">{project.name}</span>
                <span className="project-count">{project.conversations}</span>
              </span>
            )}
          </button>
        ))}
      </div>

      <div className="sidebar-footer">
        <button className={`nav-item ${screen === "settings" ? "active" : ""}`} onClick={() => onNavigate("settings")}>
          <Icon name="gear" />
          {!collapsed && <span>Settings</span>}
        </button>
        <button className="account-button" title="Local profile">
          <span className="avatar">P</span>
          {!collapsed && <span className="account-copy"><strong>Local workspace</strong><small>Private on this device</small></span>}
        </button>
      </div>
    </aside>
  );
}
