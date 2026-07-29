import { useEffect, useState } from "react";
import { cancelAgentDiscovery, discoverAgents } from "../api/agents";
import { deleteAgentOverride, getAgentOverrides, saveAgentOverride } from "../api/overrides";
import type { AgentOverride } from "../api/overrides";
import { getDisabledAgents, setAgentDisabled } from "../api/library";
import type { AgentCandidate, DetectionSignal, DiscoveryState } from "../shared/agents";
import type { BootstrapStatus } from "../shared/bootstrap";
import { I18N, SIGNAL_LABELS, type Lang, type BiStr, pick } from "../shared/i18n";

type AgentsPageProps = {
  status: BootstrapStatus;
  lang: Lang;
};

const L = (lang: Lang, s: BiStr) => pick(lang, s);

function levelOf(confidence: number): "detected" | "probable" | "possible" {
  if (confidence >= 70) return "detected";
  if (confidence >= 40) return "probable";
  return "possible";
}

function levelLabel(lang: Lang, level: string): string {
  if (level === "detected") return L(lang, I18N.levelDetected);
  if (level === "probable") return L(lang, I18N.levelProbable);
  return L(lang, I18N.levelPossible);
}

function signalLabel(lang: Lang, type: string): string {
  return SIGNAL_LABELS[type] ? pick(lang, SIGNAL_LABELS[type]) : type;
}

function AgentCard({
  agent,
  lang,
  disabled,
  onToggleDisabled,
}: {
  agent: AgentCandidate;
  lang: Lang;
  disabled: boolean;
  onToggleDisabled: (next: boolean) => void;
}) {
  const [expanded, setExpanded] = useState(false);
  const matched = agent.signals.filter((s) => s.matched).length;
  const level = levelOf(agent.confidence);

  return (
    <article className={`agent-card agent-card-${level}${disabled ? " agent-card-disabled" : ""}`}>
      <div className="agent-card-top">
        <div>
          <span className="agent-type">{agent.agentType}</span>
          <h4>{agent.displayName}</h4>
        </div>
        <span className={`confidence confidence-${level}`}>{agent.confidence}</span>
      </div>

      <div className="agent-status">
        <strong>{levelLabel(lang, level)}</strong>
        <span>{matched} {L(lang, I18N.signalsMatched)}</span>
        {disabled && <span className="tag tag-failed">{L(lang, I18N.disabledTag)}</span>}
      </div>

      <dl className="agent-meta">
        <div>
          <dt>{L(lang, I18N.skillPath)}</dt>
          <dd>
            <code title={agent.globalSkillPath ?? ""}>
              {agent.globalSkillPath ?? L(lang, I18N.notResolved)}
            </code>
          </dd>
        </div>
        <div>
          <dt>{L(lang, I18N.executable)}</dt>
          <dd>
            <code title={agent.executablePath ?? ""}>
              {agent.executablePath ?? L(lang, I18N.notResolved)}
            </code>
          </dd>
        </div>
        <div>
          <dt>{L(lang, I18N.writable)}</dt>
          <dd>
            {agent.writable === true
              ? L(lang, I18N.writableYes)
              : agent.writable === false
                ? L(lang, I18N.writableNo)
                : L(lang, I18N.writableUnknown)}
          </dd>
        </div>
      </dl>

      <div className="agent-card-actions">
        <button
          className="btn btn-sm"
          type="button"
          onClick={() => onToggleDisabled(!disabled)}
        >
          {disabled ? L(lang, I18N.enable) : L(lang, I18N.disable)}
        </button>
        <button className="expand-toggle" type="button" onClick={() => setExpanded((v) => !v)}>
          {expanded ? L(lang, I18N.hideSignals) : L(lang, I18N.showSignals)}
          <span aria-hidden="true">{expanded ? "▴" : "▾"}</span>
        </button>
      </div>

      {expanded && (
        <div className="signal-detail">
          <span className="eyebrow">{L(lang, I18N.eyebrowSignals)}</span>
          <table className="signal-table">
            <thead>
              <tr>
                <th>{L(lang, I18N.signalType)}</th>
                <th>{L(lang, I18N.signalWeight)}</th>
                <th>{L(lang, I18N.signalDetail)}</th>
              </tr>
            </thead>
            <tbody>
              {agent.signals.map((sig: DetectionSignal, i: number) => (
                <tr key={i} className={sig.matched ? "signal-row matched" : "signal-row"}>
                  <td>
                    <span className={`signal-dot ${sig.matched ? "on" : "off"}`} aria-hidden="true" />
                    {signalLabel(lang, sig.type)}
                  </td>
                  <td className="signal-weight">+{sig.weight}</td>
                  <td><code>{sig.detail ?? "—"}</code></td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </article>
  );
}

// ===== Custom Agent Form =====

const EMPTY_FORM = {
  agentType: "",
  displayName: "",
  cliName: "",
  configDir: "",
  skillDir: "",
  skillPathOverride: "",
};

function CustomAgentSection({ lang }: { lang: Lang }) {
  const [form, setForm] = useState({ ...EMPTY_FORM });
  const [overrides, setOverrides] = useState<AgentOverride[]>([]);
  const [formError, setFormError] = useState<string | null>(null);
  const [formSaved, setFormSaved] = useState(false);

  useEffect(() => {
    void getAgentOverrides().then((list) => {
      setOverrides(list.filter((o) => o.isCustom));
    });
  }, []);

  function setField(key: keyof typeof EMPTY_FORM, value: string) {
    setForm((f) => ({ ...f, [key]: value }));
    setFormError(null);
    setFormSaved(false);
  }

  async function handleSubmit() {
    if (!form.displayName.trim()) {
      setFormError(L(lang, I18N.nameRequired));
      return;
    }
    if (!form.agentType.trim()) {
      setFormError(L(lang, I18N.typeRequired));
      return;
    }

    const override: AgentOverride = {
      agentType: form.agentType.trim(),
      displayName: form.displayName.trim(),
      cliName: form.cliName.trim() || null,
      configDir: form.configDir.trim() || null,
      skillDir: form.skillDir.trim() || null,
      skillPathOverride: form.skillPathOverride.trim() || null,
      isCustom: true,
    };

    try {
      await saveAgentOverride(override);
      const updated = await getAgentOverrides();
      setOverrides(updated.filter((o) => o.isCustom));
      setForm({ ...EMPTY_FORM });
      setFormSaved(true);
      setFormError(null);
    } catch (err) {
      setFormError(err instanceof Error ? err.message : String(err));
    }
  }

  async function handleDelete(agentType: string) {
    await deleteAgentOverride(agentType);
    setOverrides((prev) => prev.filter((o) => o.agentType !== agentType));
  }

  return (
    <section className="section-block custom-agent-section">
      <div className="section-heading">
        <div>
          <span className="eyebrow">{L(lang, I18N.addCustom)}</span>
          <h3>{L(lang, I18N.customAgent)}</h3>
        </div>
      </div>

      <div className="custom-form">
        <div className="form-row">
          <label>
            <span>{L(lang, I18N.customName)} *</span>
            <input value={form.displayName} onChange={(e) => setField("displayName", e.target.value)} placeholder="My Agent" />
          </label>
          <label>
            <span>{L(lang, I18N.customType)} *</span>
            <input value={form.agentType} onChange={(e) => setField("agentType", e.target.value)} placeholder="my-agent" />
          </label>
        </div>
        <div className="form-row">
          <label>
            <span>{L(lang, I18N.customCli)}</span>
            <input value={form.cliName} onChange={(e) => setField("cliName", e.target.value)} placeholder="myagent" />
          </label>
          <label>
            <span>{L(lang, I18N.customConfigDir)}</span>
            <input value={form.configDir} onChange={(e) => setField("configDir", e.target.value)} placeholder=".myagent" />
          </label>
        </div>
        <div className="form-row">
          <label>
            <span>{L(lang, I18N.customSkillDir)}</span>
            <input value={form.skillDir} onChange={(e) => setField("skillDir", e.target.value)} placeholder=".myagent/skills" />
          </label>
          <label>
            <span>{L(lang, I18N.customSkillOverride)}</span>
            <input value={form.skillPathOverride} onChange={(e) => setField("skillPathOverride", e.target.value)} placeholder="C:\custom\path" />
          </label>
        </div>
        {formError && <div className="discovery-message error">{formError}</div>}
        {formSaved && <div className="discovery-message success">{L(lang, I18N.saved)}</div>}
        <div className="form-actions">
          <button className="primary-action" type="button" onClick={() => void handleSubmit()}>
            {L(lang, I18N.save)} <span aria-hidden="true">→</span>
          </button>
        </div>
      </div>

      <div className="custom-list">
        <span className="eyebrow">{L(lang, I18N.savedAgents)}</span>
        {overrides.length === 0 ? (
          <p className="discovery-message">{L(lang, I18N.noCustom)}</p>
        ) : (
          <div className="agent-grid agent-grid-wide">
            {overrides.map((o) => (
              <article className="agent-card" key={o.agentType}>
                <div className="agent-card-top">
                  <div>
                    <span className="agent-type">{o.agentType}</span>
                    <h4>{o.displayName}</h4>
                  </div>
                  <button className="delete-btn" type="button" onClick={() => void handleDelete(o.agentType)}>
                    {L(lang, I18N.deleteBtn)}
                  </button>
                </div>
                {o.skillPathOverride && (
                  <code title={o.skillPathOverride}>{o.skillPathOverride}</code>
                )}
              </article>
            ))}
          </div>
        )}
      </div>
    </section>
  );
}

function AgentsPage({ status, lang }: AgentsPageProps) {
  const [agents, setAgents] = useState<AgentCandidate[]>([]);
  const [discoveryState, setDiscoveryState] = useState<DiscoveryState>("idle");
  const [discoveryError, setDiscoveryError] = useState<string | null>(null);
  const [disabled, setDisabled] = useState<Set<string>>(new Set());

  useEffect(() => {
    void getDisabledAgents().then((list) => setDisabled(new Set(list))).catch(() => {});
  }, []);

  async function runDiscovery() {
    setDiscoveryState("scanning");
    setDiscoveryError(null);
    try {
      setAgents(await discoverAgents());
      setDiscoveryState("ready");
    } catch (error) {
      setDiscoveryState("error");
      setDiscoveryError(error instanceof Error ? error.message : String(error));
    }
  }

  async function toggleDisabled(agentType: string, next: boolean) {
    try {
      const list = await setAgentDisabled(agentType, next);
      setDisabled(new Set(list));
    } catch (e) {
      setDiscoveryError(String(e));
    }
  }

  const likelyCount = agents.filter((a) => a.confidence >= 40).length;

  return (
    <main className="content">
      <header className="topbar">
        <div>
          <span className="eyebrow">{L(lang, I18N.eyebrowDiscovery)}</span>
          <h1>{L(lang, I18N.agentsPageTitle)}</h1>
          <p className="topbar-sub">{L(lang, I18N.agentsPageSub)}</p>
        </div>
        <div className="version-pill">
          <span className="status-dot" aria-hidden="true" />
          v{status.version}
        </div>
      </header>

      <section className="agents-toolbar">
        <button
          className="primary-action"
          type="button"
          disabled={discoveryState === "scanning"}
          onClick={() => void runDiscovery()}
        >
          {discoveryState === "scanning"
            ? L(lang, I18N.scanning)
            : discoveryState === "ready"
              ? L(lang, I18N.rescan)
              : L(lang, I18N.discover)}
          <span aria-hidden="true">→</span>
        </button>
        {discoveryState === "scanning" && (
          <button className="cancel-action" type="button" onClick={() => void cancelAgentDiscovery()}>
            {L(lang, I18N.cancel)}
          </button>
        )}
        {discoveryState === "ready" && (
          <span className="section-meta">
            {likelyCount} {L(lang, I18N.likelyFound)} · {agents.length} {L(lang, I18N.totalAgents)}
          </span>
        )}
      </section>

      {discoveryError && (
        <div className="discovery-message error">
          {L(lang, I18N.errorPrefix)}{discoveryError}
        </div>
      )}

      {discoveryState === "idle" && (
        <div className="discovery-message">{L(lang, I18N.idleMsgAgents)}</div>
      )}

      {discoveryState === "ready" && (
        <div className="agent-grid agent-grid-wide">
          {agents.map((agent) => (
            <AgentCard
              key={agent.agentType}
              agent={agent}
              lang={lang}
              disabled={disabled.has(agent.agentType)}
              onToggleDisabled={(next) => void toggleDisabled(agent.agentType, next)}
            />
          ))}
        </div>
      )}

      <CustomAgentSection lang={lang} />
    </main>
  );
}

export default AgentsPage;
