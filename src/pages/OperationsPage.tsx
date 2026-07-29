import { useCallback, useEffect, useState } from "react";
import {
  listDeployments,
  listOperations,
  uninstallDeployment,
  verifyDeployments,
} from "../api/library";
import type { Deployment, Operation, VerifyReportItem } from "../shared/library";
import { I18N, type Lang, type BiStr, pick } from "../shared/i18n";

type Props = { lang: Lang };
const L = (lang: Lang, s: BiStr) => pick(lang, s);

export default function OperationsPage({ lang }: Props) {
  const [operations, setOperations] = useState<Operation[]>([]);
  const [deployments, setDeployments] = useState<Deployment[]>([]);
  const [verify, setVerify] = useState<VerifyReportItem[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    setError(null);
    try {
      const [ops, deps] = await Promise.all([listOperations(50), listDeployments()]);
      setOperations(ops);
      setDeployments(deps);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  async function runVerify() {
    setBusy(true);
    setError(null);
    try {
      setVerify(await verifyDeployments());
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function doUninstall(id: string, force: boolean) {
    try {
      await uninstallDeployment(id, force);
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <section className="content page page-operations">
      <header className="page-head">
        <span className="eyebrow">{L(lang, I18N.eyebrowFoundation)}</span>
        <h2>{L(lang, I18N.operationsTitle)}</h2>
        <p className="page-sub">{L(lang, I18N.operationsSub)}</p>
      </header>

      {error && <p className="page-error">{error}</p>}

      <div className="op-columns">
        <div className="op-block">
          <h3 className="section-title">
            {L(lang, I18N.operationsTitle)}
            <button type="button" className="btn btn-ghost" onClick={() => void refresh()}>
              {L(lang, I18N.refresh)}
            </button>
          </h3>
          {operations.length === 0 ? (
            <p className="muted">{L(lang, I18N.operationsEmpty)}</p>
          ) : (
            <ul className="op-list">
              {operations.map((op) => (
                <li key={op.id} className="op-row">
                  <span className={`tag tag-op tag-${op.status}`}>{op.status}</span>
                  <strong>{op.operationType}</strong>
                  <span className="muted">{op.startedAt}</span>
                  {op.errorMessage && <span className="page-error">{op.errorMessage}</span>}
                </li>
              ))}
            </ul>
          )}
        </div>

        <div className="op-block">
          <h3 className="section-title">
            {L(lang, I18N.deploy)}
            <button type="button" className="btn btn-ghost" onClick={() => void runVerify()} disabled={busy}>
              {L(lang, I18N.verifyAll)}
            </button>
          </h3>
          {deployments.length === 0 ? (
            <p className="muted">—</p>
          ) : (
            <ul className="op-list">
              {deployments.map((d) => {
                const v = verify.find((x) => x.deploymentId === d.id);
                const status = v?.status ?? d.status;
                return (
                  <li key={d.id} className="op-row">
                    <span className={`tag tag-${status}`}>{status}</span>
                    <code>{d.targetPath}</code>
                    <button
                      type="button"
                      className="btn btn-danger btn-sm"
                      onClick={() => void doUninstall(d.id, false)}
                    >
                      {L(lang, I18N.deleteBtn)}
                    </button>
                  </li>
                );
              })}
            </ul>
          )}
        </div>
      </div>
    </section>
  );
}
