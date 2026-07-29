import { useEffect, useState } from "react";
import { discoverAgents } from "../api/agents";
import { importSkillFromDirectory } from "../api/library";
import type { AgentCandidate } from "../shared/agents";
import { I18N, type Lang, type BiStr, pick } from "../shared/i18n";
import type { PageId } from "../shared/useNavigation";

type Props = {
  lang: Lang;
  onNavigate: (page: PageId) => void;
  onClose: () => void;
};

const L = (lang: Lang, s: BiStr) => pick(lang, s);

const STORAGE_KEY = "skillark.firstRunDone";

export function firstRunDismissed(): boolean {
  try {
    return localStorage.getItem(STORAGE_KEY) === "1";
  } catch {
    return false;
  }
}

export function dismissFirstRun() {
  try {
    localStorage.setItem(STORAGE_KEY, "1");
  } catch {
    // ignore storage failures (private mode, etc.)
  }
}

const STEPS = 3;

export default function FirstRunWizard({ lang, onNavigate, onClose }: Props) {
  const [step, setStep] = useState(0);
  const [agents, setAgents] = useState<AgentCandidate[] | null>(null);
  const [scanning, setScanning] = useState(false);
  const [importPath, setImportPath] = useState("");
  const [importing, setImporting] = useState(false);
  const [imported, setImported] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Auto-scan on entering the scan step.
  useEffect(() => {
    if (step === 1 && agents === null) {
      void runScan();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [step]);

  async function runScan() {
    setScanning(true);
    setError(null);
    try {
      setAgents(await discoverAgents({}));
    } catch (e) {
      setError(String(e));
    } finally {
      setScanning(false);
    }
  }

  async function doImport() {
    if (!importPath.trim()) return;
    setImporting(true);
    setError(null);
    try {
      await importSkillFromDirectory(importPath.trim());
      setImported(true);
    } catch (e) {
      setError(String(e));
    } finally {
      setImporting(false);
    }
  }

  function finish() {
    dismissFirstRun();
    onClose();
    onNavigate("library");
  }

  function skip() {
    dismissFirstRun();
    onClose();
  }

  const detected = (agents ?? []).filter((a) => a.confidence >= 40);

  return (
    <div className="wizard-overlay" role="dialog" aria-modal="true">
      <div className="wizard">
        <header className="wizard-head">
          <span className="eyebrow">{L(lang, I18N.eyebrowFoundation)}</span>
          <h2>SkillArk · {L(lang, I18N.brandSub)}</h2>
          <button type="button" className="wizard-close" onClick={skip} aria-label="Close">
            ✕
          </button>
        </header>

        <ol className="wizard-progress">
          {Array.from({ length: STEPS }).map((_, i) => (
            <li key={i} className={i === step ? "active" : i < step ? "done" : ""}>
              {i + 1}
            </li>
          ))}
        </ol>

        {error && <p className="page-error">{error}</p>}

        {step === 0 && (
          <div className="wizard-body">
            <p>{L(lang, I18N.heroDesc)}</p>
            <ul className="wizard-bullets">
              <li>{L(lang, I18N.librarySub)}</li>
              <li>{L(lang, I18N.deploySub)}</li>
              <li>{L(lang, I18N.sidebarNote)}</li>
            </ul>
          </div>
        )}

        {step === 1 && (
          <div className="wizard-body">
            <p>{L(lang, I18N.agentsPageSub)}</p>
            <button type="button" className="btn" onClick={() => void runScan()} disabled={scanning}>
              {scanning ? L(lang, I18N.scanning) : L(lang, I18N.rescan)}
            </button>
            <p className="muted">
              {detected.length} {L(lang, I18N.likelyFound)} · {(agents ?? []).length}{" "}
              {L(lang, I18N.totalAgents)}
            </p>
            <ul className="wizard-agents">
              {(agents ?? []).slice(0, 6).map((a) => (
                <li key={a.agentType}>
                  <strong>{a.displayName}</strong>
                  <span className={`tag tag-${a.confidence >= 70 ? "synced" : "modified"}`}>
                    {a.confidence}
                  </span>
                </li>
              ))}
            </ul>
          </div>
        )}

        {step === 2 && (
          <div className="wizard-body">
            <p>{L(lang, I18N.librarySub)}</p>
            {imported ? (
              <p className="result-ok">{L(lang, I18N.saved)}</p>
            ) : (
              <div className="import-row">
                <label className="field">
                  <span>{L(lang, I18N.importDir)}</span>
                  <input
                    type="text"
                    value={importPath}
                    onChange={(e) => setImportPath(e.target.value)}
                    placeholder={L(lang, I18N.pathPlaceholderDir)}
                  />
                </label>
                <button
                  type="button"
                  className="btn btn-primary"
                  onClick={() => void doImport()}
                  disabled={importing || !importPath.trim()}
                >
                  {importing ? L(lang, I18N.importing) : L(lang, I18N.importDir)}
                </button>
              </div>
            )}
          </div>
        )}

        <footer className="wizard-foot">
          <button type="button" className="btn btn-ghost" onClick={skip}>
            {L(lang, I18N.cancel)}
          </button>
          <div className="wizard-foot-right">
            {step > 0 && (
              <button type="button" className="btn" onClick={() => setStep((s) => s - 1)}>
                ‹
              </button>
            )}
            {step < STEPS - 1 && (
              <button
                type="button"
                className="btn btn-primary"
                onClick={() => setStep((s) => s + 1)}
              >
                {L(lang, I18N.discover)} ›
              </button>
            )}
            {step === STEPS - 1 && (
              <button type="button" className="btn btn-primary" onClick={finish}>
                {L(lang, I18N.libraryTitle)} ›
              </button>
            )}
          </div>
        </footer>
      </div>
    </div>
  );
}
