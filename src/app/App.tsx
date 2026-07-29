import { useEffect, useState } from "react";
import { loadBootstrapStatus } from "../api/bootstrap";
import { listOperations, listSkills } from "../api/library";
import { browserFallbackStatus, type BootstrapStatus } from "../shared/bootstrap";
import { I18N, type Lang, type BiStr, pick } from "../shared/i18n";
import { useNavigation, type PageId } from "../shared/useNavigation";
import HomePage from "../pages/HomePage";
import AgentsPage from "../pages/AgentsPage";
import LibraryPage from "../pages/LibraryPage";
import DeployPage from "../pages/DeployPage";
import OperationsPage from "../pages/OperationsPage";
import WorkspacesPage from "../pages/WorkspacesPage";
import FirstRunWizard, { firstRunDismissed } from "../pages/FirstRunWizard";
import "./App.css";

const L = (lang: Lang, s: BiStr) => pick(lang, s);

// Nav index → page. null entries stay disabled ("coming soon").
const NAV_PAGE_BY_INDEX: (PageId | null)[] = [
  "overview",
  "library",
  "agents",
  "workspaces",
  "operations",
];

function App() {
  const [status, setStatus] = useState<BootstrapStatus>(browserFallbackStatus);
  const nav = useNavigation();
  const [deploySkill, setDeploySkill] = useState<{ id: string; canonical: string } | null>(null);
  const [showWizard, setShowWizard] = useState(false);

  useEffect(() => {
    void loadBootstrapStatus().then(setStatus);
  }, []);

  // First-run detection: show the wizard once, on a genuinely empty library.
  useEffect(() => {
    if (firstRunDismissed()) return;
    void Promise.all([listSkills(), listOperations(1)])
      .then(([skills, ops]) => {
        if (skills.length === 0 && ops.length === 0) {
          setShowWizard(true);
        }
      })
      .catch(() => {
        // Backend not ready (e.g. browser preview) — skip the wizard.
      });
  }, []);

  const lang = nav.lang;
  const navLabels = I18N.nav[lang];

  const goToDeploy = (id: string, canonical: string) => {
    setDeploySkill({ id, canonical });
    nav.navigate("deploy");
  };

  return (
    <div className="shell">
      <aside className="sidebar">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true">S</span>
          <div>
            <strong>SkillArk</strong>
            <span>{L(lang, I18N.brandSub)}</span>
          </div>
        </div>

        <nav aria-label="Primary">
          {navLabels.map((label, index) => {
            const pageId = NAV_PAGE_BY_INDEX[index];
            const isActive = pageId !== null && nav.page === pageId;
            const disabled = pageId === null;
            return (
              <button
                className={`nav-item${isActive ? " active" : ""}`}
                key={label}
                type="button"
                disabled={disabled}
                title={disabled ? L(lang, I18N.comingSoon) : label}
                onClick={() => pageId && nav.navigate(pageId)}
              >
                <span aria-hidden="true">{I18N.navIcons[index]}</span>
                {label}
                {disabled && <span className="nav-badge">M2</span>}
              </button>
            );
          })}
        </nav>

        <div className="sidebar-note">
          <span className="eyebrow">{L(lang, I18N.eyebrowLocal)}</span>
          <p>{L(lang, I18N.sidebarNote)}</p>
          <button
            className="lang-toggle"
            type="button"
            onClick={() => nav.setLang(lang === "en" ? "zh" : "en")}
            title={L(lang, I18N.langHint)}
          >
            <span className="lang-icon" aria-hidden="true">🌐</span>
            <span className="lang-current">{lang === "zh" ? "中文" : "English"}</span>
            <span className="lang-switch">⇄</span>
          </button>
        </div>
      </aside>

      {nav.page === "overview" && <HomePage status={status} lang={lang} />}
      {nav.page === "library" && (
        <LibraryPage lang={lang} onDeploy={goToDeploy} />
      )}
      {nav.page === "agents" && <AgentsPage status={status} lang={lang} />}
      {nav.page === "workspaces" && <WorkspacesPage lang={lang} />}
      {nav.page === "deploy" && (
        <DeployPage
          lang={lang}
          initialSkillId={deploySkill?.id}
          initialCanonicalName={deploySkill?.canonical}
        />
      )}
      {nav.page === "operations" && <OperationsPage lang={lang} />}

      {showWizard && (
        <FirstRunWizard
          lang={lang}
          onNavigate={(page: PageId) => nav.navigate(page)}
          onClose={() => setShowWizard(false)}
        />
      )}
    </div>
  );
}

export default App;
