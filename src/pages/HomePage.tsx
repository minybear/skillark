import { useState } from "react";
import { cancelAgentDiscovery, discoverAgents } from "../api/agents";
import type { AgentCandidate, DiscoveryState } from "../shared/agents";
import type { BootstrapStatus } from "../shared/bootstrap";
import { I18N, type Lang, type BiStr, pick } from "../shared/i18n";

type HomePageProps = { status: BootstrapStatus; lang: Lang };

const L = (lang: Lang, s: BiStr) => pick(lang, s);

const FOUNDATIONS = [
  { en: "Tauri 2 + React", zh: "Tauri 2 + React", descEn: "Official desktop scaffold", descZh: "官方桌面应用骨架" },
  { en: "Rust domain core", zh: "Rust 领域核心", descEn: "Pure domain boundaries", descZh: "纯净的领域边界" },
  { en: "SQLite migrations", zh: "SQLite 迁移", descEn: "Versioned local storage", descZh: "版本化的本地存储" },
  { en: "JSON contract tests", zh: "JSON 契约测试", descEn: "Schema-backed DTOs", descZh: "Schema 驱动的 DTO" },
];

const MILESTONES = [
  { labelEn: "Design freeze", labelZh: "设计冻结", detailEn: "Contracts and boundaries", detailZh: "契约与边界", done: true },
  { labelEn: "Agent detection", labelZh: "代理探测", detailEn: "Multi-signal Windows scan", detailZh: "多信号 Windows 扫描", done: true },
  { labelEn: "Import & deploy", labelZh: "导入与分发", detailEn: "Copy, Junction, recovery", detailZh: "复制、链接、恢复", done: true },
  { labelEn: "MVP interface", labelZh: "MVP 界面", detailEn: "The complete local workflow", detailZh: "完整的本地工作流", done: true },
];

function HomePage({ status, lang }: HomePageProps) {
  const [agents, setAgents] = useState<AgentCandidate[]>([]);
  const [discoveryState, setDiscoveryState] = useState<DiscoveryState>("idle");
  const [discoveryError, setDiscoveryError] = useState<string | null>(null);

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

  return (
    <main className="content">
      <header className="topbar">
        <div>
          <span className="eyebrow">{L(lang, I18N.eyebrowProject)}</span>
          <h1>{L(lang, I18N.h1)}</h1>
        </div>
        <div className="version-pill">
          <span className="status-dot" aria-hidden="true" />
          v{status.version}
        </div>
      </header>

      <section className="hero">
        <div className="hero-copy">
          <span className="phase">{status.phase}</span>
          <h2>{L(lang, I18N.vaultReady)}</h2>
          <p>{L(lang, I18N.heroDesc)}</p>
          <div className="hero-actions">
            <button
              className="primary-action"
              type="button"
              disabled={discoveryState === "scanning"}
              onClick={() => void runDiscovery()}
            >
              {discoveryState === "scanning" ? L(lang, I18N.scanning) : L(lang, I18N.discover)}
              <span aria-hidden="true">→</span>
            </button>
            {discoveryState === "scanning" ? (
              <button className="cancel-action" type="button" onClick={() => void cancelAgentDiscovery()}>
                {L(lang, I18N.cancel)}
              </button>
            ) : (
              <span>{L(lang, I18N.next)} {status.nextMilestone}</span>
            )}
          </div>
        </div>
        <div className="vault" aria-label={L(lang, I18N.vaultTitle)}>
          <div className="vault-ring outer" />
          <div className="vault-ring middle" />
          <div className="vault-core">
            <span>4</span>
            <small>{L(lang, I18N.vaultFoundations)}</small>
          </div>
        </div>
      </section>

      <section className="section-block">
        <div className="section-heading">
          <div>
            <span className="eyebrow">{L(lang, I18N.eyebrowFoundation)}</span>
            <h3>{L(lang, I18N.foundationReady)}</h3>
          </div>
          <span className="section-meta">{L(lang, I18N.foundationMeta)}</span>
        </div>
        <div className="foundation-grid">
          {FOUNDATIONS.map((f, i) => (
            <article className="foundation-card" key={f.en}>
              <span className="card-index">0{i + 1}</span>
              <div className="check" aria-hidden="true">✓</div>
              <h4>{lang === "zh" ? f.zh : f.en}</h4>
              <p>{lang === "zh" ? f.descZh : f.descEn}</p>
            </article>
          ))}
        </div>
      </section>

      <section className="section-block discovery-block">
        <div className="section-heading">
          <div>
            <span className="eyebrow">{L(lang, I18N.eyebrowDiscovery)}</span>
            <h3>{L(lang, I18N.agentsHeading)}</h3>
          </div>
          <span className="section-meta">
            {discoveryState === "ready"
              ? `${agents.filter((a) => a.confidence >= 40).length} ${L(lang, I18N.likelyFound)}`
              : L(lang, I18N.notScanned)}
          </span>
        </div>

        {discoveryError && (
          <div className="discovery-message error">
            {L(lang, I18N.errorPrefix)}{discoveryError}
          </div>
        )}

        {discoveryState === "idle" && (
          <div className="discovery-message">{L(lang, I18N.idleMsg)}</div>
        )}

        {discoveryState === "ready" && (
          <div className="agent-grid">
            {agents.map((agent) => {
              const matchedSignals = agent.signals.filter((s) => s.matched).length;
              const level = agent.confidence >= 70 ? "detected" : agent.confidence >= 40 ? "probable" : "possible";
              const levelLabel = level === "detected" ? L(lang, I18N.levelDetected) : level === "probable" ? L(lang, I18N.levelProbable) : L(lang, I18N.levelPossible);
              return (
                <article className="agent-card" key={agent.agentType}>
                  <div className="agent-card-top">
                    <div>
                      <span className="agent-type">{agent.agentType}</span>
                      <h4>{agent.displayName}</h4>
                    </div>
                    <span className={`confidence confidence-${level}`}>{agent.confidence}</span>
                  </div>
                  <div className="agent-status">
                    <strong>{levelLabel}</strong>
                    <span>{matchedSignals} {L(lang, I18N.signalsMatched)}</span>
                  </div>
                  <code title={agent.globalSkillPath ?? undefined}>
                    {agent.globalSkillPath ?? L(lang, I18N.noPath)}
                  </code>
                </article>
              );
            })}
          </div>
        )}
      </section>

      <section className="roadmap">
        <div className="section-heading">
          <div>
            <span className="eyebrow">{L(lang, I18N.eyebrowRoadmap)}</span>
            <h3>{L(lang, I18N.roadmapHeading)}</h3>
          </div>
        </div>
        <div className="milestone-list">
          {MILESTONES.map((m, i) => (
            <div className={m.done ? "milestone done" : "milestone"} key={m.labelEn}>
              <span className="milestone-number">{m.done ? "✓" : i + 1}</span>
              <div>
                <strong>{lang === "zh" ? m.labelZh : m.labelEn}</strong>
                <span>{lang === "zh" ? m.detailZh : m.detailEn}</span>
              </div>
            </div>
          ))}
        </div>
      </section>
    </main>
  );
}

export default HomePage;
