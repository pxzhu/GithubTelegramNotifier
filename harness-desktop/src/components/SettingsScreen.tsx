import { useState } from "react";

import type { HardwareProfile, Provider } from "../types";
import { Icon, type IconName } from "./Icon";

type SettingsSection = "general" | "appearance" | "providers" | "models" | "routing" | "agents" | "local" | "components" | "usage" | "projects" | "security" | "advanced";

const settingsNav: Array<{ id: SettingsSection; label: string; icon: IconName; badge?: string; divider?: boolean }> = [
  { id: "general", label: "General", icon: "gear" },
  { id: "appearance", label: "Appearance", icon: "sparkles" },
  { id: "providers", label: "Providers", icon: "layers", divider: true },
  { id: "models", label: "Models", icon: "bolt" },
  { id: "routing", label: "Routing", icon: "branch" },
  { id: "agents", label: "Agents", icon: "user" },
  { id: "local", label: "Local AI", icon: "terminal", divider: true },
  { id: "components", label: "Components", icon: "grid" },
  { id: "usage", label: "Usage & budgets", icon: "activity" },
  { id: "projects", label: "Projects", icon: "folder", divider: true },
  { id: "security", label: "Security", icon: "warning" },
  { id: "advanced", label: "Advanced", icon: "code" },
];

function Toggle({ checked: initial = false, label, disabled = false }: { checked?: boolean; label: string; disabled?: boolean }) {
  const [checked, setChecked] = useState(initial);
  return <button type="button" role="switch" aria-checked={checked} aria-label={label} disabled={disabled} className={`toggle ${checked ? "on" : ""}`} onClick={() => setChecked((value) => !value)}><span /></button>;
}

function SettingsRow({ title, description, children }: { title: string; description?: string; children: React.ReactNode }) {
  return <div className="settings-row"><div><strong>{title}</strong>{description && <p>{description}</p>}</div><div className="settings-control">{children}</div></div>;
}

function StatusPill({ status }: { status: Provider["status"] }) {
  const label = status === "connected" ? "Connected" : status === "ready" ? "Ready" : status === "update" ? "Update available" : status === "login-required" ? "Login required" : "Unavailable";
  return <span className={`provider-status ${status}`}><span />{label}</span>;
}

function ProvidersSettings({ providers }: { providers: Provider[] }) {
  const [selectedId, setSelectedId] = useState(providers[0]?.id);
  const selected = providers.find((provider) => provider.id === selectedId) ?? providers[0];
  return (
    <div className="settings-detail providers-settings">
      <div className="settings-page-heading"><div><h1>Providers</h1><p>Detected provider status and the capabilities Harness can route work to.</p></div><button className="primary-button" disabled title="Provider installation is not connected in this foundation build"><Icon name="plus" size={14} /> Add provider</button></div>
      <div className="provider-settings-layout">
        <div className="provider-card-list">
          {providers.map((provider) => <button key={provider.id} className={`provider-list-card ${provider.id === selected?.id ? "active" : ""}`} onClick={() => setSelectedId(provider.id)}><span className="provider-logo" style={{ "--provider-color": provider.color } as React.CSSProperties}>{provider.shortName}</span><span><strong>{provider.name}</strong><StatusPill status={provider.status} /></span><Icon name="arrow" size={14} /></button>)}
          <button className="provider-list-card add" disabled><span className="provider-logo"><Icon name="plus" size={16} /></span><span><strong>Custom CLI agent</strong><small>Use the reference JSON-RPC adapter</small></span></button>
        </div>
        {selected && <section className="provider-detail-card">
          <header><span className="provider-logo large" style={{ "--provider-color": selected.color } as React.CSSProperties}>{selected.shortName}</span><div><h2>{selected.name}</h2><StatusPill status={selected.status} /></div><Toggle key={selected.id} checked={selected.status !== "unavailable"} disabled label={`Enable ${selected.name}`} /></header>
          <p className="provider-description">{selected.description}</p>
          <div className="provider-facts"><div><span>Version</span><strong>{selected.version}</strong></div><div><span>Install source</span><strong>{selected.source}</strong></div><div><span>Default model</span><strong>{selected.model}</strong></div></div>
          <div className="capability-section"><h3>Capabilities</h3><div>{selected.capabilities.map((capability) => <span key={capability}>{capability}</span>)}</div></div>
          <div className="settings-subsection"><h3>Configuration</h3><SettingsRow title="Default model"><label className="settings-select"><select defaultValue={selected.model}><option>{selected.model}</option><option>Auto</option></select><Icon name="chevron" size={11} /></label></SettingsRow><SettingsRow title="Concurrency" description="Maximum simultaneous runs"><label className="settings-select small"><select defaultValue="2"><option>1</option><option>2</option><option>4</option></select><Icon name="chevron" size={11} /></label></SettingsRow><SettingsRow title="Workspace access" description="Limited to the active project by default"><button className="secondary-button">Manage</button></SettingsRow></div>
          <footer><span className="provider-readonly-note">Status is detected by Harness Core.</span><button className="secondary-button" disabled><Icon name="refresh" size={13} /> Check on next launch</button></footer>
        </section>}
      </div>
    </div>
  );
}

function LocalAISettings({ hardware }: { hardware: HardwareProfile }) {
  const tierDescription = hardware.tier === 0
    ? "Local inference is not recommended on this hardware profile."
    : hardware.tier <= 2
      ? "A small CPU-efficient model may be suitable for routing and summaries."
      : "This profile may support a medium local model after runtime compatibility checks.";
  return <div className="settings-detail local-ai-settings">
    <div className="settings-page-heading"><div><h1>Local AI</h1><p>Run lightweight routing and summarization privately on this computer.</p></div><span className="ready-badge off"><span />Not installed</span></div>
    <section className="hardware-card"><header><div><span className="hardware-icon"><Icon name="terminal" /></span><div><span>This computer</span><h2>{hardware.device}</h2></div></div><span className="ready-badge"><span />Analysis current</span></header><div className="hardware-grid"><div><span>Processor</span><strong>{hardware.cpu}</strong></div><div><span>Memory</span><strong>{hardware.memory}</strong></div><div><span>Graphics</span><strong>{hardware.gpu}</strong></div><div><span>Storage</span><strong>{hardware.freeDisk}</strong></div></div><div className="hardware-tier"><span>Hardware tier {hardware.tier}</span><p>{tierDescription}</p></div></section>
    <section className="recommended-model-card"><span className="recommended-ribbon"><Icon name="sparkles" size={12} /> Installation gate</span><div className="model-hero"><span className="local-model-orb">AI</span><div><span>Managed local runtime</span><h2>No signed model catalog configured</h2><p>A production catalog must provide a compatible runtime, model license, download hash, memory limits, and platform support before installation.</p></div><span className="model-size">Not installed</span></div><div className="model-purpose-row"><span><Icon name="branch" size={14} /> Routing</span><span><Icon name="message" size={14} /> Summaries</span><span><Icon name="check" size={14} /> Review</span></div><div className="model-runtime"><span><strong>Runtime</strong> Not installed</span><span><strong>Catalog</strong> Required</span><span><strong>Integrity</strong> Required</span></div><footer><span /><button className="primary-button" disabled><Icon name="download" size={14} /> Signed catalog required</button></footer></section>
    <SettingsRow title="Offline mode" description="Disable cloud providers and keep all work on this device"><Toggle label="Offline mode" /></SettingsRow>
  </div>;
}

function ComponentsSettings({ providers }: { providers: Provider[] }) {
  const components = [
    { id: "app", name: "Harness Desktop", kind: "APP", version: "0.1.0", source: "Development build", policy: "Manual", update: false },
    ...providers.map((provider) => ({ id: provider.id, name: provider.name, kind: "PROVIDER", version: provider.version, source: provider.source, policy: "Notify only", update: provider.status === "update" })),
  ];
  const updateCount = components.filter((component) => component.update).length;
  return <div className="settings-detail components-settings">
    <div className="settings-page-heading"><div><h1>Components</h1><p>Locally detected versions and their installation authority.</p></div><div><button className="secondary-button" disabled><Icon name="refresh" size={13} /> Signed update feed not configured</button></div></div>
    <div className="component-summary"><div><span className="summary-check"><Icon name="check" /></span><span><strong>{components.length} components discovered locally</strong><small>Version checks do not change package-manager state</small></span></div><div><strong>{updateCount}</strong><span>reported updates</span></div></div>
    <section className="component-table"><div className="component-table-head"><span>Component</span><span>Version</span><span>Source</span><span>Policy</span><span>Status</span></div>{components.map((component) => <div className="component-row" key={component.id}><span><i className={`component-icon ${component.kind.toLocaleLowerCase()}`}><Icon name={component.kind === "APP" ? "sparkles" : "layers"} size={15} /></i><span><strong>{component.name}</strong><small>{component.kind}</small></span></span><span>{component.version}</span><span>{component.source}</span><span>{component.policy}</span><span>{component.update ? <em className="external-state">Reported update</em> : <em className="external-state">Not checked</em>}</span></div>)}</section>
    <div className="component-policy-note"><Icon name="warning" size={16} /><span><strong>External package managers stay in control</strong><p>Harness will notify you about Homebrew and vendor-managed updates, but will not alter them automatically.</p></span></div>
  </div>;
}

function GeneralSettings({ section }: { section: SettingsSection }) {
  const titles: Partial<Record<SettingsSection, [string, string]>> = {
    general: ["General", "Choose how Harness behaves across projects."], appearance: ["Appearance", "Make Harness feel at home on this system."], models: ["Models", "Create aliases and choose defaults by capability."], routing: ["Routing", "Balance quality, speed, privacy and provider usage."], agents: ["Agents", "Configure roles, concurrency and review policies."], usage: ["Usage & budgets", "Set quota guardrails and usage notifications."], projects: ["Projects", "Defaults for workspaces and project storage."], security: ["Security", "Control tool approvals, credentials and data handling."], advanced: ["Advanced", "Diagnostics, developer features and experimental behavior."],
  };
  const [title, description] = titles[section] ?? ["Settings", "Configure Harness."];
  return <div className="settings-detail generic-settings"><div className="settings-page-heading"><div><h1>{title}</h1><p>{description}</p></div></div>
    {section === "appearance" && <section className="settings-section-card"><h2>Theme</h2><div className="theme-options"><button className="active"><span className="theme-preview system"><i /><i /></span><strong>System</strong></button><button><span className="theme-preview light"><i /></span><strong>Light</strong></button><button><span className="theme-preview dark"><i /></span><strong>Dark</strong></button></div><SettingsRow title="Use platform-native materials" description="Respect macOS vibrancy and Windows backdrop effects"><Toggle checked label="Native materials" /></SettingsRow></section>}
    {section === "routing" && <><section className="settings-section-card"><h2>Default routing mode</h2><div className="routing-mode-grid"><button><span>✦</span><strong>Quality First</strong><small>Best available provider</small></button><button className="active"><span>◐</span><strong>Balanced</strong><small>Quality and usage</small></button><button><span>◒</span><strong>Claude Saver</strong><small>Minimize Claude calls</small></button><button><span>⌁</span><strong>Local First</strong><small>Prioritize this device</small></button></div></section><section className="settings-section-card"><h2>Agent decisions</h2><SettingsRow title="Explain provider selection" description="Show a concise reason in Agent Activity"><Toggle checked label="Explain provider selection" /></SettingsRow><SettingsRow title="Use reviewer for code changes" description="Run an independent review when risk is medium or higher"><Toggle checked label="Use reviewer" /></SettingsRow><SettingsRow title="Maximum parallel agents"><label className="settings-select"><select defaultValue="3"><option>1</option><option>2</option><option>3</option><option>4</option></select><Icon name="chevron" size={11} /></label></SettingsRow></section></>}
    {section === "security" && <><section className="security-banner"><Icon name="check" /><div><strong>Credential boundaries are enforced</strong><p>Harness does not read Claude or Codex credential files. Microsoft sign-in remains disabled until an operating-system secure store is configured.</p></div></section><section className="settings-section-card"><h2>Tool permissions</h2><SettingsRow title="Read files" description="Safe actions inside a project workspace"><span className="policy-label allow">Allow</span></SettingsRow><SettingsRow title="Modify files" description="Editing and creating workspace files"><label className="settings-select"><select defaultValue="Ask once per run"><option>Always ask</option><option>Ask once per run</option><option>Allow</option></select><Icon name="chevron" size={11} /></label></SettingsRow><SettingsRow title="External actions" description="Send messages or change remote systems"><span className="policy-label ask">Always ask</span></SettingsRow><SettingsRow title="Destructive commands" description="Deletion or hard-to-reverse operations"><span className="policy-label block">Always ask</span></SettingsRow></section></>}
    {!['appearance','routing','security'].includes(section) && <><section className="settings-section-card"><h2>Behavior</h2><SettingsRow title="Restore windows on launch" description="Continue from the last project and conversation"><Toggle checked label="Restore windows" /></SettingsRow><SettingsRow title="Background activity" description="Allow running tasks to continue when the window is hidden"><Toggle checked label="Background activity" /></SettingsRow><SettingsRow title="Desktop notifications" description="Notify when a run finishes or requires approval"><Toggle checked label="Desktop notifications" /></SettingsRow></section><section className="settings-section-card"><h2>Data</h2><SettingsRow title="Local data directory" description="Conversation history, indexes and run metadata"><button className="secondary-button">Reveal in Finder</button></SettingsRow><SettingsRow title="Export Harness archive"><button className="secondary-button"><Icon name="download" size={13} /> Export</button></SettingsRow></section></>}
  </div>;
}

interface SettingsScreenProps { providers: Provider[]; hardware: HardwareProfile; }

export function SettingsScreen({ providers, hardware }: SettingsScreenProps) {
  const [section, setSection] = useState<SettingsSection>("providers");
  return <main className="settings-screen"><aside className="settings-nav"><header><h1>Settings</h1></header><nav>{settingsNav.map((item) => <div key={item.id} className={item.divider ? "settings-nav-divider" : ""}><button className={section === item.id ? "active" : ""} onClick={() => setSection(item.id)}><Icon name={item.icon} size={16} /><span>{item.label}</span>{item.badge && <em>{item.badge}</em>}</button></div>)}</nav><footer><span>Harness 0.1.0</span><small>Local-first · Development build</small></footer></aside><div className="settings-content">{section === "providers" ? <ProvidersSettings providers={providers} /> : section === "local" ? <LocalAISettings hardware={hardware} /> : section === "components" ? <ComponentsSettings providers={providers} /> : <GeneralSettings section={section} />}</div></main>;
}
