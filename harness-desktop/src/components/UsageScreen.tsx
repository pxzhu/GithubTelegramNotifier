import { useState } from "react";

import type { UsageDatum } from "../types";
import { Icon } from "./Icon";

interface UsageScreenProps {
  usage: UsageDatum[];
}

export function UsageScreen({ usage }: UsageScreenProps) {
  const [range, setRange] = useState("30 days");
  const totalRequests = usage.reduce((sum, datum) => sum + datum.requests, 0);
  const tokenReportingProviders = usage.filter((datum) => datum.tokens).length;
  const costReportingProviders = usage.filter((datum) => datum.cost).length;
  return (
    <main className="screen-page usage-screen">
      <div className="page-toolbar usage-toolbar"><div><span className="page-eyebrow">Insights</span><h1>Usage</h1><p>Understand where your agents spend time and quota.</p></div><div className="page-toolbar-actions"><label className="date-range"><Icon name="activity" size={14} /><select value={range} onChange={(event) => setRange(event.target.value)} aria-label="Date range"><option>7 days</option><option>30 days</option><option>3 months</option></select><Icon name="chevron" size={11} /></label><button className="secondary-button" disabled title="Usage export is not connected in this foundation build"><Icon name="download" size={14} /> Export</button></div></div>

      <section className="usage-metric-grid">
        <div className="metric-card primary"><span>Provider requests</span><strong>{totalRequests}</strong><small>Only provider-reported requests are counted.</small><span className="metric-icon blue"><Icon name="activity" /></span></div>
        <div className="metric-card"><span>Active providers</span><strong>{usage.length}</strong><small>Providers with locally stored usage events</small><span className="metric-icon blue"><Icon name="layers" /></span></div>
        <div className="metric-card"><span>Token reporting</span><strong>{tokenReportingProviders}</strong><small>Unknown token counts remain blank</small><span className="metric-icon green"><Icon name="message" /></span></div>
        <div className="metric-card"><span>Cost reporting</span><strong>{costReportingProviders}</strong><small>No price-based estimates are invented</small><span className="metric-icon violet"><Icon name="check" /></span></div>
      </section>

      <div className="usage-content-grid">
        <section className="dashboard-card provider-breakdown">
          <header><div><h2>Provider breakdown</h2><p>Share of known activity</p></div><button className="icon-button"><Icon name="more" /></button></header>
          <div className="large-stacked-bar">{usage.map((datum) => <span key={datum.provider} style={{ width: `${datum.percent}%`, background: datum.color }} />)}</div>
          <div className="provider-usage-table">
            <div className="usage-table-head"><span>Provider</span><span>Requests</span><span>Tokens</span><span>Est. cost</span></div>
            {usage.map((datum) => (
              <div className="usage-table-row" key={datum.provider}><span><i style={{ background: datum.color }} />{datum.provider}<small>{datum.percent}%</small></span><span>{datum.requests}</span><span>{datum.tokens ?? "—"}</span><span>{datum.cost ?? "Unavailable"}</span></div>
            ))}
            {usage.length === 0 && <div className="empty-usage-row"><strong>No usage recorded</strong><span>Run data will appear after a configured provider reports it.</span></div>}
          </div>
        </section>

        <section className="dashboard-card savings-breakdown">
          <header><div><h2>Routing efficiency</h2><p>Derived only from completed task outcomes</p></div></header>
          <div className="usage-placeholder"><span><Icon name="branch" size={20} /></span><strong>Not enough task history</strong><p>Claude avoidance, delegation, retry, and first-pass metrics remain unavailable until actual Agent Runs are recorded.</p></div>
        </section>

        <section className="dashboard-card project-breakdown">
          <header><div><h2>By project</h2><p>Known usage dimensions</p></div></header>
          <div className="usage-placeholder compact"><span><Icon name="folder" size={20} /></span><strong>No project breakdown yet</strong><p>Project-level rows require usage events that include a project identifier.</p></div>
        </section>

        <section className="dashboard-card data-note"><span className="data-note-icon"><Icon name="info" /></span><div><h3>Usage is provider-reported</h3><p>Harness never invents unavailable token counts. When recorded, Microsoft 365 request counts can appear while unavailable token usage stays blank.</p></div></section>
      </div>
    </main>
  );
}
