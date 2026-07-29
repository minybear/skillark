import { useCallback, useEffect, useState } from "react";
import {
  createProjectWorkspace,
  deleteProjectWorkspace,
  listWorkspaces,
} from "../api/library";
import type { Workspace } from "../shared/library";
import { I18N, type Lang, type BiStr, pick } from "../shared/i18n";

type Props = { lang: Lang };
const L = (lang: Lang, s: BiStr) => pick(lang, s);

export default function WorkspacesPage({ lang }: Props) {
  const [workspaces, setWorkspaces] = useState<Workspace[]>([]);
  const [name, setName] = useState("");
  const [root, setRoot] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const refresh = useCallback(async () => {
    setError(null);
    try {
      setWorkspaces(await listWorkspaces());
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  async function create() {
    if (!name.trim()) return;
    setBusy(true);
    setError(null);
    try {
      await createProjectWorkspace(name.trim(), root.trim());
      setName("");
      setRoot("");
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function remove(id: string) {
    try {
      await deleteProjectWorkspace(id);
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  }

  const globalWs = workspaces.filter((w) => w.kind === "global");
  const projects = workspaces.filter((w) => w.kind === "project");

  return (
    <section className="content page page-workspaces">
      <header className="page-head">
        <span className="eyebrow">{L(lang, I18N.eyebrowFoundation)}</span>
        <h2>{L(lang, I18N.workspacesTitle)}</h2>
        <p className="page-sub">{L(lang, I18N.workspacesSub)}</p>
      </header>

      {error && <p className="page-error">{error}</p>}

      <div className="import-row">
        <label className="field">
          <span>{L(lang, I18N.projectName)}</span>
          <input type="text" value={name} onChange={(e) => setName(e.target.value)} />
        </label>
        <label className="field">
          <span>{L(lang, I18N.projectRoot)}</span>
          <input
            type="text"
            value={root}
            onChange={(e) => setRoot(e.target.value)}
            placeholder={L(lang, I18N.projectRootPh)}
          />
        </label>
        <button type="button" className="btn btn-primary" onClick={() => void create()} disabled={busy || !name.trim()}>
          {L(lang, I18N.create)}
        </button>
      </div>

      <h3 className="section-title">{L(lang, I18N.globalWs)}</h3>
      <ul className="lib-list">
        {globalWs.map((w) => (
          <li key={w.id} className="lib-row">
            <div className="lib-row-main">
              <strong>{w.name}</strong>
              <span className="lib-meta"><span className="tag tag-synced">global</span></span>
            </div>
          </li>
        ))}
      </ul>

      <h3 className="section-title">
        {L(lang, I18N.workspacesTitle)}
        <button type="button" className="btn btn-ghost" onClick={() => void refresh()}>
          {L(lang, I18N.refresh)}
        </button>
      </h3>
      {projects.length === 0 ? (
        <p className="muted">{L(lang, I18N.noWorkspaces)}</p>
      ) : (
        <ul className="lib-list">
          {projects.map((w) => (
            <li key={w.id} className="lib-row">
              <div className="lib-row-main">
                <strong>{w.name}</strong>
                <code className="lib-canonical">{w.rootPath ?? "—"}</code>
                <span className="lib-meta">
                  <span className={`tag tag-${w.status === "missing" ? "failed" : "synced"}`}>
                    {w.status === "missing" ? L(lang, I18N.missing) : "available"}
                  </span>
                </span>
              </div>
              <div className="lib-row-actions">
                <button type="button" className="btn btn-danger" onClick={() => void remove(w.id)}>
                  {L(lang, I18N.deleteBtn)}
                </button>
              </div>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
