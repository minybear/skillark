import { useCallback, useEffect, useState } from "react";
import {
  deleteSkill,
  getSkillDetail,
  importSkillFromDirectory,
  importSkillFromZip,
  listSkills,
} from "../api/library";
import type { Skill, SkillDetail } from "../shared/library";
import { I18N, type Lang, type BiStr, pick } from "../shared/i18n";

type Props = {
  lang: Lang;
  onDeploy: (skillId: string, canonicalName: string) => void;
};

const L = (lang: Lang, s: BiStr) => pick(lang, s);

function shortHash(h: string | null): string {
  if (!h) return "—";
  return h.slice(0, 10);
}

export default function LibraryPage({ lang, onDeploy }: Props) {
  const [skills, setSkills] = useState<Skill[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [dirPath, setDirPath] = useState("");
  const [zipPath, setZipPath] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [detail, setDetail] = useState<SkillDetail | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setSkills(await listSkills());
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  async function doImport(kind: "dir" | "zip", path: string) {
    if (!path.trim()) return;
    setBusy(kind);
    setError(null);
    try {
      if (kind === "dir") {
        await importSkillFromDirectory(path.trim());
      } else {
        await importSkillFromZip(path.trim());
      }
      if (kind === "dir") setDirPath("");
      else setZipPath("");
      await refresh();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(null);
    }
  }

  async function doDelete(skill: Skill) {
    if (!window.confirm(L(lang, I18N.confirmDelete))) return;
    try {
      await deleteSkill(skill.id);
      await refresh();
    } catch (e) {
      setError(String(e));
    }
  }

  async function openDetail(skill: Skill) {
    setError(null);
    try {
      setDetail(await getSkillDetail(skill.id));
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <section className="content page page-library">
      <header className="page-head">
        <span className="eyebrow">{L(lang, I18N.eyebrowFoundation)}</span>
        <h2>{L(lang, I18N.libraryTitle)}</h2>
        <p className="page-sub">{L(lang, I18N.librarySub)}</p>
      </header>

      <div className="import-row">
        <label className="field">
          <span>{L(lang, I18N.importDir)}</span>
          <input
            type="text"
            value={dirPath}
            onChange={(e) => setDirPath(e.target.value)}
            placeholder={L(lang, I18N.pathPlaceholderDir)}
          />
        </label>
        <button
          type="button"
          className="btn btn-primary"
          disabled={busy !== null || !dirPath.trim()}
          onClick={() => doImport("dir", dirPath)}
        >
          {busy === "dir" ? L(lang, I18N.importing) : L(lang, I18N.importDir)}
        </button>
      </div>

      <div className="import-row">
        <label className="field">
          <span>{L(lang, I18N.importZip)}</span>
          <input
            type="text"
            value={zipPath}
            onChange={(e) => setZipPath(e.target.value)}
            placeholder={L(lang, I18N.pathPlaceholderZip)}
          />
        </label>
        <button
          type="button"
          className="btn"
          disabled={busy !== null || !zipPath.trim()}
          onClick={() => doImport("zip", zipPath)}
        >
          {busy === "zip" ? L(lang, I18N.importing) : L(lang, I18N.importZip)}
        </button>
      </div>

      {error && <p className="page-error">{error}</p>}

      <div className="lib-section">
        <h3 className="section-title">
          {L(lang, I18N.libraryTitle)}
          <button type="button" className="btn btn-ghost" onClick={() => void refresh()}>
            {L(lang, I18N.refresh)}
          </button>
        </h3>

        {loading ? (
          <p className="muted">{L(lang, I18N.loading)}</p>
        ) : skills.length === 0 ? (
          <p className="muted">{L(lang, I18N.libraryEmpty)}</p>
        ) : (
          <ul className="lib-list">
            {skills.map((skill) => (
              <li key={skill.id} className="lib-row">
                <div className="lib-row-main">
                  <strong>{skill.displayName}</strong>
                  <code className="lib-canonical">{skill.canonicalName}</code>
                  <span className="lib-meta">
                    {skill.versionLabel && <span>{L(lang, I18N.versionLabel)}: {skill.versionLabel}</span>}
                    <span>{L(lang, I18N.hashShort)}: {shortHash(skill.contentHash)}</span>
                    <span className={`tag tag-${skill.status}`}>{skill.status}</span>
                  </span>
                </div>
                <div className="lib-row-actions">
                  <button
                    type="button"
                    className="btn"
                    onClick={() => void openDetail(skill)}
                  >
                    {L(lang, I18N.view)}
                  </button>
                  <button
                    type="button"
                    className="btn btn-primary"
                    onClick={() => onDeploy(skill.id, skill.canonicalName)}
                  >
                    {L(lang, I18N.deploy)}
                  </button>
                  <button
                    type="button"
                    className="btn btn-danger"
                    onClick={() => void doDelete(skill)}
                  >
                    {L(lang, I18N.deleteBtn)}
                  </button>
                </div>
              </li>
            ))}
          </ul>
        )}
      </div>

      {detail && (
        <div className="wizard-overlay" role="dialog" aria-modal="true" onClick={() => setDetail(null)}>
          <div className="wizard skill-detail" onClick={(e) => e.stopPropagation()}>
            <header className="wizard-head">
              <span className="eyebrow">{L(lang, I18N.libraryTitle)}</span>
              <h2>{detail.displayName}</h2>
              <button type="button" className="wizard-close" onClick={() => setDetail(null)} aria-label="Close">
                ✕
              </button>
            </header>
            <div className="wizard-body">
              <p className="muted">
                <code>{detail.canonicalName}</code>
                {detail.versionLabel && <span> · {detail.versionLabel}</span>}
                {detail.contentHash && <span> · {shortHash(detail.contentHash)}</span>}
              </p>
              {detail.description && <p>{detail.description}</p>}
              <h3 className="section-title">{L(lang, I18N.filesHeading)}</h3>
              <ul className="lib-list">
                {detail.files.map((f) => (
                  <li key={f.path} className="op-row">
                    <code>{f.path}</code>
                    <span className="muted">{f.size} B</span>
                  </li>
                ))}
              </ul>
              {detail.skillMd && (
                <>
                  <h3 className="section-title">{L(lang, I18N.skillMdHeading)}</h3>
                  <pre className="skill-md">{detail.skillMd}</pre>
                </>
              )}
            </div>
            <footer className="wizard-foot">
              <button type="button" className="btn" onClick={() => setDetail(null)}>
                {L(lang, I18N.cancel)}
              </button>
            </footer>
          </div>
        </div>
      )}
    </section>
  );
}
