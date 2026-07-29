import { useCallback, useEffect, useMemo, useState } from "react";
import { discoverAgents } from "../api/agents";
import {
  executeDeployment,
  getDisabledAgents,
  listSkills,
  listWorkspaces,
  planDeployment,
  resolveGlobalTargetPaths,
  resolveProjectTargetPaths,
} from "../api/library";
import type {
  ExecutionReport,
  InstallMode,
  Plan,
  PlanTargetSpec,
  Skill,
  Workspace,
} from "../shared/library";
import type { AgentCandidate } from "../shared/agents";
import { I18N, type Lang, type BiStr, pick } from "../shared/i18n";

type Props = {
  lang: Lang;
  initialSkillId?: string;
  initialCanonicalName?: string;
};

const L = (lang: Lang, s: BiStr) => pick(lang, s);

export default function DeployPage({ lang, initialSkillId, initialCanonicalName }: Props) {
  const [skills, setSkills] = useState<Skill[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(initialSkillId ?? null);
  const [canonicalName, setCanonicalName] = useState<string>(initialCanonicalName ?? "");
  const [agents, setAgents] = useState<AgentCandidate[]>([]);
  const [scanning, setScanning] = useState(false);
  const [selectedAgents, setSelectedAgents] = useState<Set<string>>(new Set());
  const [mode, setMode] = useState<InstallMode>("copy");
  const [plan, setPlan] = useState<Plan | null>(null);
  const [report, setReport] = useState<ExecutionReport | null>(null);
  const [lastExecutedPlan, setLastExecutedPlan] = useState<Plan | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [scope, setScope] = useState<"global" | "project">("global");
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [selectedProjectId, setSelectedProjectId] = useState<string | null>(null);
  const [disabled, setDisabled] = useState<Set<string>>(new Set());

  useEffect(() => {
    void listSkills().then(setSkills).catch((e) => setError(String(e)));
    void listWorkspaces().then(setWorkspaces).catch((e) => setError(String(e)));
    void getDisabledAgents().then((list) => setDisabled(new Set(list))).catch(() => {});
  }, []);

  const selectedSkill = useMemo(
    () => skills.find((s) => s.id === selectedId) ?? null,
    [skills, selectedId],
  );

  const projectWorkspaces = useMemo(
    () => workspaces.filter((w) => w.kind === "project" && w.rootPath),
    [workspaces],
  );

  const selectedProject = useMemo(
    () => projectWorkspaces.find((w) => w.id === selectedProjectId) ?? null,
    [projectWorkspaces, selectedProjectId],
  );

  const writableAgents = useMemo(
    () =>
      agents.filter(
        (a) => a.writable === true && a.globalSkillPath && !disabled.has(a.agentType),
      ),
    [agents, disabled],
  );

  const rescan = useCallback(async () => {
    setScanning(true);
    setError(null);
    try {
      setAgents(await discoverAgents({}));
    } catch (e) {
      setError(String(e));
    } finally {
      setScanning(false);
    }
  }, []);

  function selectSkill(id: string) {
    setSelectedId(id);
    const sk = skills.find((s) => s.id === id);
    setCanonicalName(sk?.canonicalName ?? "");
    setPlan(null);
    setReport(null);
  }

  function toggleAgent(agentType: string) {
    setSelectedAgents((prev) => {
      const next = new Set(prev);
      if (next.has(agentType)) next.delete(agentType);
      else next.add(agentType);
      return next;
    });
    setPlan(null);
    setReport(null);
  }

  async function buildPlan() {
    if (!selectedSkill) {
      setError(L(lang, I18N.selectSkillFirst));
      return;
    }
    if (scope === "project" && !selectedProject) {
      setError(L(lang, I18N.selectSkillFirst));
      return;
    }
    setBusy(true);
    setError(null);
    setReport(null);
    try {
      const chosenAgentTypes = writableAgents
        .filter((a) => selectedAgents.has(a.agentType))
        .map((a) => a.agentType);

      let targets: PlanTargetSpec[];
      if (scope === "project" && selectedProject) {
        const resolved = await resolveProjectTargetPaths(
          canonicalName,
          selectedProject.rootPath ?? "",
          chosenAgentTypes,
        );
        targets = resolved.map((r) => ({
          agentId: r.agentType,
          workspaceId: selectedProject.id,
          targetPath: r.targetPath,
          mode,
        }));
      } else {
        const resolved = await resolveGlobalTargetPaths(canonicalName, chosenAgentTypes);
        targets = resolved.map((r) => ({
          agentId: r.agentType,
          workspaceId: "global-default",
          targetPath: r.targetPath,
          mode,
        }));
      }

      const built = await planDeployment({ skillVersionId: selectedSkill.id, targets });
      setPlan(built);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function execute() {
    if (!plan) return;
    const executingPlan = plan;
    setBusy(true);
    setError(null);
    try {
      setReport(await executeDeployment(executingPlan));
      setLastExecutedPlan(executingPlan);
      setPlan(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  const failedJunctionTargets = useMemo(() => {
    if (!report || !lastExecutedPlan) return [];
    const failed = new Set(
      report.outcomes
        .filter((outcome) => !outcome.ok)
        .map(
          (outcome) =>
            `${outcome.agentId}\u0000${outcome.workspaceId}\u0000${outcome.targetPath}`,
        ),
    );
    return lastExecutedPlan.targets.filter(
      (target) =>
        target.mode === "junction" &&
        failed.has(`${target.agentId}\u0000${target.workspaceId}\u0000${target.targetPath}`),
    );
  }, [lastExecutedPlan, report]);

  async function buildCopyRetryPlan() {
    if (!lastExecutedPlan || failedJunctionTargets.length === 0) return;
    setBusy(true);
    setError(null);
    try {
      const targets: PlanTargetSpec[] = failedJunctionTargets.map((target) => ({
        agentId: target.agentId,
        workspaceId: target.workspaceId,
        targetPath: target.targetPath,
        mode: "copy",
      }));
      const built = await planDeployment({
        skillVersionId: lastExecutedPlan.skillVersionId,
        targets,
      });
      setMode("copy");
      setPlan(built);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <section className="content page page-deploy">
      <header className="page-head">
        <span className="eyebrow">{L(lang, I18N.eyebrowFoundation)}</span>
        <h2>{L(lang, I18N.deployTitle)}</h2>
        <p className="page-sub">{L(lang, I18N.deploySub)}</p>
      </header>

      {error && <p className="page-error">{error}</p>}

      <fieldset className="step">
        <legend>{L(lang, I18N.pickSkill)}</legend>
        <select value={selectedId ?? ""} onChange={(e) => selectSkill(e.target.value)}>
          <option value="" disabled>
            —
          </option>
          {skills.map((s) => (
            <option key={s.id} value={s.id}>
              {s.displayName} ({s.canonicalName})
            </option>
          ))}
        </select>
      </fieldset>

      <fieldset className="step">
        <legend>{L(lang, I18N.pickAgents)}</legend>
        <button type="button" className="btn" onClick={() => void rescan()} disabled={scanning}>
          {scanning ? L(lang, I18N.scanning) : L(lang, I18N.scanAgents)}
        </button>
        {writableAgents.length === 0 ? (
          <p className="muted">{L(lang, I18N.noWritableAgents)}</p>
        ) : (
          <ul className="pick-list">
            {writableAgents.map((a) => (
              <li key={a.agentType}>
                <label>
                  <input
                    type="checkbox"
                    checked={selectedAgents.has(a.agentType)}
                    onChange={() => toggleAgent(a.agentType)}
                  />
                  <span>
                    <strong>{a.displayName}</strong>
                    <code>{a.globalSkillPath}</code>
                  </span>
                </label>
              </li>
            ))}
          </ul>
        )}
      </fieldset>

      <fieldset className="step">
        <legend>{L(lang, I18N.chooseScope)}</legend>
        <label className="radio">
          <input
            type="radio"
            name="scope"
            checked={scope === "global"}
            onChange={() => {
              setScope("global");
              setPlan(null);
              setReport(null);
            }}
          />
          {L(lang, I18N.globalWs)}
        </label>
        <label className="radio">
          <input
            type="radio"
            name="scope"
            checked={scope === "project"}
            onChange={() => {
              setScope("project");
              setPlan(null);
              setReport(null);
            }}
          />
          {L(lang, I18N.workspacesTitle)}
        </label>
        {scope === "project" && (
          <select
            value={selectedProjectId ?? ""}
            onChange={(e) => {
              setSelectedProjectId(e.target.value || null);
              setPlan(null);
              setReport(null);
            }}
          >
            <option value="" disabled>
              —
            </option>
            {projectWorkspaces.map((w) => (
              <option key={w.id} value={w.id}>
                {w.name} ({w.rootPath})
              </option>
            ))}
          </select>
        )}
      </fieldset>

      <fieldset className="step">
        <legend>{L(lang, I18N.chooseMode)}</legend>
        <label className="radio">
          <input
            type="radio"
            name="mode"
            checked={mode === "copy"}
            onChange={() => setMode("copy")}
          />
          {L(lang, I18N.modeCopy)}
        </label>
        <label className="radio">
          <input
            type="radio"
            name="mode"
            checked={mode === "junction"}
            onChange={() => setMode("junction")}
          />
          {L(lang, I18N.modeJunction)}
        </label>
      </fieldset>

      <div className="step-actions">
        <button
          type="button"
          className="btn"
          onClick={() => void buildPlan()}
          disabled={busy || !selectedSkill || selectedAgents.size === 0}
        >
          {L(lang, I18N.buildPlan)}
        </button>
      </div>

      {plan && (
        <div className="plan-review">
          <h3>{L(lang, I18N.buildPlan)}</h3>
          {plan.requiresConfirmation && (
            <p className="page-warn">{L(lang, I18N.requiresConfirmation)}</p>
          )}
          <table className="plan-table">
            <thead>
              <tr>
                <th>{L(lang, I18N.targetPath)}</th>
                <th>{L(lang, I18N.conflictCol)}</th>
              </tr>
            </thead>
            <tbody>
              {plan.targets.map((t) => (
                <tr key={`${t.agentId}:${t.targetPath}`}>
                  <td><code>{t.targetPath}</code></td>
                  <td><span className={`tag tag-conflict tag-${t.conflict}`}>{t.conflict}</span></td>
                </tr>
              ))}
            </tbody>
          </table>
          <button
            type="button"
            className="btn btn-primary"
            onClick={() => void execute()}
            disabled={busy}
          >
            {L(lang, I18N.executePlan)}
          </button>
        </div>
      )}

      {report && (
        <div className="plan-review">
          <h3>{L(lang, I18N.resultsHeading)}</h3>
          <p className="muted">
            {report.succeeded} {L(lang, I18N.succeeded)} · {report.failed} {L(lang, I18N.failed)}
          </p>
          <ul className="result-list">
            {report.outcomes.map((o) => (
              <li key={`${o.agentId}:${o.targetPath}`} className={o.ok ? "result-ok" : "result-fail"}>
                <code>{o.targetPath}</code>
                <span>{o.ok ? L(lang, I18N.ok) : o.error}</span>
              </li>
            ))}
          </ul>
          {failedJunctionTargets.length > 0 && (
            <div className="junction-fallback">
              <p className="page-warn">{L(lang, I18N.junctionFallbackNotice)}</p>
              <button
                type="button"
                className="btn"
                onClick={() => void buildCopyRetryPlan()}
                disabled={busy}
              >
                {L(lang, I18N.buildCopyRetryPlan)}
              </button>
            </div>
          )}
        </div>
      )}
    </section>
  );
}
