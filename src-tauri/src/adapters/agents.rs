use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    domain::agent::{AgentCandidate, AgentKind, DetectionSignal},
    ports::{AgentAdapter, DetectionContext, ValidationResult},
};

const CLI_WEIGHT: i32 = 40;
const CONFIG_WEIGHT: i32 = 25;
const SKILL_WEIGHT: i32 = 25;
const PROCESS_WEIGHT: i32 = 10;
const USER_WEIGHT: i32 = 100;

#[derive(Clone, Copy)]
pub struct AgentSpec {
    pub kind: BuiltInAgent,
    pub display_name: &'static str,
    pub cli_names: &'static [&'static str],
    pub process_names: &'static [&'static str],
    pub config_dir: &'static str,
    pub skill_dir: &'static str,
}

#[derive(Clone, Copy)]
pub enum BuiltInAgent {
    ClaudeCode,
    Cursor,
    Codex,
    WorkBuddy,
}

impl BuiltInAgent {
    fn kind(self) -> AgentKind {
        match self {
            Self::ClaudeCode => AgentKind::ClaudeCode,
            Self::Cursor => AgentKind::Cursor,
            Self::Codex => AgentKind::Codex,
            Self::WorkBuddy => AgentKind::WorkBuddy,
        }
    }
}

pub struct BuiltInAgentAdapter {
    spec: AgentSpec,
}

impl BuiltInAgentAdapter {
    pub const fn new(spec: AgentSpec) -> Self {
        Self { spec }
    }

    fn config_path(&self, context: &DetectionContext) -> PathBuf {
        context.home_dir.join(self.spec.config_dir)
    }

    fn default_skill_path(&self, context: &DetectionContext) -> PathBuf {
        context.home_dir.join(self.spec.skill_dir)
    }
}

impl AgentAdapter for BuiltInAgentAdapter {
    fn kind(&self) -> AgentKind {
        self.spec.kind.kind()
    }

    fn display_name(&self) -> String {
        self.spec.display_name.to_owned()
    }

    fn detect(&self, context: &DetectionContext) -> Vec<AgentCandidate> {
        let kind = self.kind();
        let contract_value = kind.as_contract_value();
        let executable_path = find_executable(context, self.spec.cli_names);
        let config_path = self.config_path(context);
        let default_skill_path = self.default_skill_path(context);
        let manual_path = context.manual_skill_paths.get(contract_value).cloned();
        let global_skill_path = manual_path
            .as_ref()
            .cloned()
            .unwrap_or_else(|| default_skill_path.clone());
        let process_matched = context.running_processes.iter().any(|running| {
            self.spec.process_names.iter().any(|candidate| {
                running.eq_ignore_ascii_case(candidate)
                    || running
                        .strip_suffix(".exe")
                        .is_some_and(|stem| stem.eq_ignore_ascii_case(candidate))
            })
        });

        let mut signals = vec![
            signal(
                "path_executable",
                executable_path.is_some(),
                CLI_WEIGHT,
                executable_path.as_deref(),
            ),
            signal(
                "config_directory",
                config_path.is_dir(),
                CONFIG_WEIGHT,
                Some(&config_path),
            ),
            signal(
                "skill_directory",
                default_skill_path.is_dir(),
                SKILL_WEIGHT,
                Some(&default_skill_path),
            ),
            DetectionSignal {
                signal_type: "running_process".to_owned(),
                matched: process_matched,
                weight: PROCESS_WEIGHT,
                detail: None,
            },
        ];

        if let Some(path) = manual_path.as_deref() {
            signals.push(signal("user_override", true, USER_WEIGHT, Some(path)));
        }

        let confidence = signals
            .iter()
            .filter(|signal| signal.matched)
            .map(|signal| signal.weight)
            .sum::<i32>()
            .clamp(0, 100) as u8;

        vec![AgentCandidate {
            kind,
            display_name: self.display_name().to_owned(),
            confidence,
            executable_path,
            global_skill_path: Some(normalize_path(&global_skill_path)),
            writable: path_writable(&global_skill_path),
            signals,
        }]
    }

    fn validate_configuration(&self, candidate: &AgentCandidate) -> ValidationResult {
        let writable = candidate.writable.unwrap_or(false);
        let mut warnings = Vec::new();

        if candidate.global_skill_path.is_none() {
            warnings.push("No global Skill directory is configured.".to_owned());
        }
        if !writable {
            warnings.push("The configured Skill directory is not writable.".to_owned());
        }

        ValidationResult {
            valid: candidate.global_skill_path.is_some() && writable,
            writable,
            warnings,
        }
    }

    fn global_skill_path(&self, candidate: &AgentCandidate) -> Option<PathBuf> {
        candidate.global_skill_path.clone()
    }

    fn project_skill_path(
        &self,
        _candidate: &AgentCandidate,
        project_root: &Path,
    ) -> Option<PathBuf> {
        Some(normalize_path(
            &project_root.join(self.spec.skill_dir.trim_start_matches(['/', '\\'])),
        ))
    }
}

pub fn built_in_adapters() -> Vec<Box<dyn AgentAdapter>> {
    vec![
        Box::new(BuiltInAgentAdapter::new(AgentSpec {
            kind: BuiltInAgent::ClaudeCode,
            display_name: "Claude Code",
            cli_names: &["claude"],
            process_names: &["claude"],
            config_dir: ".claude",
            skill_dir: ".claude/skills",
        })),
        Box::new(BuiltInAgentAdapter::new(AgentSpec {
            kind: BuiltInAgent::Cursor,
            display_name: "Cursor",
            cli_names: &["cursor"],
            process_names: &["cursor"],
            config_dir: ".cursor",
            skill_dir: ".cursor/skills",
        })),
        Box::new(BuiltInAgentAdapter::new(AgentSpec {
            kind: BuiltInAgent::Codex,
            display_name: "Codex",
            cli_names: &["codex"],
            process_names: &["codex"],
            config_dir: ".codex",
            skill_dir: ".codex/skills",
        })),
        Box::new(BuiltInAgentAdapter::new(AgentSpec {
            kind: BuiltInAgent::WorkBuddy,
            display_name: "WorkBuddy",
            cli_names: &["workbuddy"],
            process_names: &["workbuddy"],
            config_dir: ".workbuddy",
            skill_dir: ".workbuddy/skills",
        })),
    ]
}

fn find_executable(context: &DetectionContext, names: &[&str]) -> Option<PathBuf> {
    let extensions: &[&str] = if cfg!(windows) {
        &["", ".exe", ".cmd", ".bat"]
    } else {
        &[""]
    };

    context.path_entries.iter().find_map(|directory| {
        names.iter().find_map(|name| {
            extensions.iter().find_map(|extension| {
                let path = directory.join(format!("{name}{extension}"));
                path.is_file().then(|| normalize_path(&path))
            })
        })
    })
}

fn signal(signal_type: &str, matched: bool, weight: i32, detail: Option<&Path>) -> DetectionSignal {
    DetectionSignal {
        signal_type: signal_type.to_owned(),
        matched,
        weight,
        detail: detail.map(|path| normalize_path(path).to_string_lossy().into_owned()),
    }
}

pub(super) fn normalize_path(path: &Path) -> PathBuf {
    let normalized = path.canonicalize().unwrap_or_else(|_| {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            std::env::current_dir()
                .map(|current| current.join(path))
                .unwrap_or_else(|_| path.to_path_buf())
        }
    });

    #[cfg(windows)]
    {
        let value = normalized.to_string_lossy();
        if let Some(unc_path) = value.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{unc_path}"));
        }
        if let Some(local_path) = value.strip_prefix(r"\\?\") {
            return PathBuf::from(local_path);
        }
    }

    normalized
}

pub(super) fn path_writable(path: &Path) -> Option<bool> {
    let existing = path.ancestors().find(|candidate| candidate.exists())?;
    fs::metadata(existing)
        .ok()
        .map(|metadata| !metadata.permissions().readonly())
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        fs,
        path::{Path, PathBuf},
    };

    use super::{AgentSpec, BuiltInAgent, BuiltInAgentAdapter};
    use crate::ports::{AgentAdapter, DetectionContext};

    fn context(home: &Path) -> DetectionContext {
        DetectionContext {
            home_dir: home.to_path_buf(),
            app_data: None,
            local_app_data: None,
            program_files: None,
            program_files_x86: None,
            path_entries: vec![],
            running_processes: vec![],
            manual_skill_paths: HashMap::new(),
        }
    }

    fn codex_adapter() -> BuiltInAgentAdapter {
        BuiltInAgentAdapter::new(AgentSpec {
            kind: BuiltInAgent::Codex,
            display_name: "Codex",
            cli_names: &["codex"],
            process_names: &["codex"],
            config_dir: ".codex",
            skill_dir: ".codex/skills",
        })
    }

    fn temporary_home(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("skillark-{name}-{}", std::process::id()));
        if path.exists() {
            fs::remove_dir_all(&path).expect("stale test directory should be removable");
        }
        fs::create_dir_all(&path).expect("test directory should be created");
        path
    }

    #[test]
    fn scores_multiple_detection_signals() {
        let home = temporary_home("multi-signal");
        let bin = home.join("bin");
        fs::create_dir_all(home.join(".codex/skills")).unwrap();
        fs::create_dir_all(&bin).unwrap();
        fs::write(
            bin.join(if cfg!(windows) { "codex.cmd" } else { "codex" }),
            b"",
        )
        .unwrap();

        let mut detection_context = context(&home);
        detection_context.path_entries.push(bin);
        detection_context
            .running_processes
            .push("codex.exe".to_owned());

        let candidate = codex_adapter().detect(&detection_context).remove(0);

        assert_eq!(candidate.confidence, 100);
        assert!(candidate.executable_path.is_some());
        assert_eq!(
            candidate.global_skill_path,
            Some(home.join(".codex/skills"))
        );
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn manual_path_has_priority_and_full_confidence() {
        let home = temporary_home("manual");
        let manual = home.join("自定义技能");
        fs::create_dir_all(&manual).unwrap();
        let mut detection_context = context(&home);
        detection_context
            .manual_skill_paths
            .insert("codex".to_owned(), manual.clone());

        let candidate = codex_adapter().detect(&detection_context).remove(0);

        assert_eq!(candidate.confidence, 100);
        assert_eq!(candidate.global_skill_path, Some(manual));
        assert!(candidate
            .signals
            .iter()
            .any(|signal| signal.signal_type == "user_override" && signal.matched));
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn missing_agent_is_a_zero_confidence_candidate_not_an_error() {
        let home = temporary_home("missing");
        let candidate = codex_adapter().detect(&context(&home)).remove(0);

        assert_eq!(candidate.confidence, 0);
        assert!(candidate.executable_path.is_none());
        fs::remove_dir_all(home).unwrap();
    }
}
