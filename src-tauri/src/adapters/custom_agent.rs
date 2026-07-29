use std::path::{Path, PathBuf};

use crate::{
    domain::agent::{AgentCandidate, AgentKind, DetectionSignal},
    ports::{AgentAdapter, DetectionContext, ValidationResult},
};

use super::agents::{normalize_path, path_writable};

const CLI_WEIGHT: i32 = 40;
const CONFIG_WEIGHT: i32 = 25;
const SKILL_WEIGHT: i32 = 25;
const PROCESS_WEIGHT: i32 = 10;
const USER_WEIGHT: i32 = 100;

pub struct CustomAgentAdapter {
    pub agent_type: String,
    pub display_name: String,
    pub cli_name: Option<String>,
    pub config_dir: Option<String>,
    pub skill_dir: Option<String>,
    pub skill_path_override: Option<PathBuf>,
}

impl CustomAgentAdapter {
    fn config_path(&self, context: &DetectionContext) -> Option<PathBuf> {
        self.config_dir
            .as_ref()
            .map(|dir| context.home_dir.join(dir))
    }

    fn default_skill_path(&self, context: &DetectionContext) -> Option<PathBuf> {
        self.skill_dir
            .as_ref()
            .map(|dir| context.home_dir.join(dir))
    }
}

impl AgentAdapter for CustomAgentAdapter {
    fn kind(&self) -> AgentKind {
        AgentKind::Custom(self.agent_type.clone())
    }

    fn display_name(&self) -> String {
        self.display_name.clone()
    }

    fn detect(&self, context: &DetectionContext) -> Vec<AgentCandidate> {
        let kind = self.kind();
        let contract_value = kind.as_contract_value().to_owned();

        let cli_names_owned: Vec<String> = self.cli_name.iter().cloned().collect();
        let cli_refs: Vec<&str> = cli_names_owned.iter().map(String::as_str).collect();
        let executable_path = if cli_refs.is_empty() {
            None
        } else {
            find_executable(context, &cli_refs)
        };

        let config_path = self.config_path(context);
        let default_skill_path = self.default_skill_path(context);
        let manual_path = context
            .manual_skill_paths
            .get(&contract_value)
            .cloned()
            .or_else(|| self.skill_path_override.clone());

        let global_skill_path = manual_path
            .clone()
            .or_else(|| default_skill_path.clone());

        let process_matched = self
            .cli_name
            .as_deref()
            .map(|needle| {
                context.running_processes.iter().any(|running| {
                    running.eq_ignore_ascii_case(needle)
                        || running
                            .strip_suffix(".exe")
                            .is_some_and(|stem| stem.eq_ignore_ascii_case(needle))
                })
            })
            .unwrap_or(false);

        let mut signals = vec![
            signal(
                "path_executable",
                executable_path.is_some(),
                CLI_WEIGHT,
                executable_path.as_deref(),
            ),
            signal_opt(
                "config_directory",
                config_path.as_ref().map(|p| p.is_dir()).unwrap_or(false),
                CONFIG_WEIGHT,
                config_path.as_deref(),
            ),
            signal_opt(
                "skill_directory",
                default_skill_path.as_ref().map(|p| p.is_dir()).unwrap_or(false),
                SKILL_WEIGHT,
                default_skill_path.as_deref(),
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
            display_name: self.display_name.clone(),
            confidence,
            executable_path,
            writable: global_skill_path.as_deref().and_then(path_writable),
            global_skill_path: global_skill_path.map(|p| normalize_path(&p)),
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
        self.skill_dir.as_ref().map(|dir| {
            normalize_path(&project_root.join(dir.trim_start_matches(['/', '\\'])))
        })
    }
}

pub fn custom_adapters(
    overrides: &[crate::application::agent_overrides::AgentOverride],
) -> Vec<Box<dyn AgentAdapter>> {
    overrides
        .iter()
        .filter(|o| o.is_custom)
        .map(|o| {
            Box::new(CustomAgentAdapter {
                agent_type: o.agent_type.clone(),
                display_name: o.display_name.clone(),
                cli_name: o.cli_name.clone(),
                config_dir: o.config_dir.clone(),
                skill_dir: o.skill_dir.clone(),
                skill_path_override: o.skill_path_override.clone().map(PathBuf::from),
            }) as Box<dyn AgentAdapter>
        })
        .collect()
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

fn signal(
    signal_type: &str,
    matched: bool,
    weight: i32,
    detail: Option<&Path>,
) -> DetectionSignal {
    DetectionSignal {
        signal_type: signal_type.to_owned(),
        matched,
        weight,
        detail: detail.map(|path| normalize_path(path).to_string_lossy().into_owned()),
    }
}

fn signal_opt(
    signal_type: &str,
    matched: bool,
    weight: i32,
    detail: Option<&Path>,
) -> DetectionSignal {
    signal(signal_type, matched, weight, detail)
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, fs, path::PathBuf};

    use super::CustomAgentAdapter;
    use crate::ports::{AgentAdapter, DetectionContext};

    fn context(home: &std::path::Path) -> DetectionContext {
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

    fn adapter_full() -> CustomAgentAdapter {
        CustomAgentAdapter {
            agent_type: "myagent".to_owned(),
            display_name: "My Agent".to_owned(),
            cli_name: Some("myagent".to_owned()),
            config_dir: Some(".myagent".to_owned()),
            skill_dir: Some(".myagent/skills".to_owned()),
            skill_path_override: None,
        }
    }

    fn temporary_home(name: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "skillark-custom-{name}-{}",
            std::process::id()
        ));
        if path.exists() {
            fs::remove_dir_all(&path).expect("stale test directory should be removable");
        }
        fs::create_dir_all(&path).expect("test directory should be created");
        path
    }

    #[test]
    fn full_signal_match_scores_hundred() {
        let home = temporary_home("full");
        let bin = home.join("bin");
        fs::create_dir_all(home.join(".myagent/skills")).unwrap();
        fs::create_dir_all(&bin).unwrap();
        fs::write(
            bin.join(if cfg!(windows) { "myagent.cmd" } else { "myagent" }),
            b"",
        )
        .unwrap();

        let mut detection_context = context(&home);
        detection_context.path_entries.push(bin);
        detection_context
            .running_processes
            .push("myagent.exe".to_owned());

        let candidate = adapter_full().detect(&detection_context).remove(0);
        assert_eq!(candidate.confidence, 100);
        assert!(candidate.executable_path.is_some());
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn no_signal_scores_zero() {
        let home = temporary_home("none");
        let candidate = adapter_full().detect(&context(&home)).remove(0);
        assert_eq!(candidate.confidence, 0);
        assert!(candidate.executable_path.is_none());
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn skill_path_override_scores_hundred_and_uses_path() {
        let home = temporary_home("override");
        let custom_dir = home.join("自定义Agent/skills");
        fs::create_dir_all(&custom_dir).unwrap();

        let adapter = CustomAgentAdapter {
            agent_type: "myagent".to_owned(),
            display_name: "My Agent".to_owned(),
            cli_name: None,
            config_dir: None,
            skill_dir: None,
            skill_path_override: Some(custom_dir.clone()),
        };

        let candidate = adapter.detect(&context(&home)).remove(0);
        assert_eq!(candidate.confidence, 100);
        assert_eq!(candidate.global_skill_path, Some(custom_dir));
        fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn chinese_paths_work_correctly() {
        let home = temporary_home("chinese");
        let custom_skill = home.join("自定义Agent/skills");
        fs::create_dir_all(&custom_skill).unwrap();

        let adapter = CustomAgentAdapter {
            agent_type: "我的代理".to_owned(),
            display_name: "自定义代理".to_owned(),
            cli_name: Some("mycli".to_owned()),
            config_dir: Some("自定义Agent".to_owned()),
            skill_dir: Some("自定义Agent/skills".to_owned()),
            skill_path_override: None,
        };

        let candidate = adapter.detect(&context(&home)).remove(0);
        assert_eq!(candidate.confidence, 50);
        let resolved = candidate.global_skill_path.as_deref().unwrap().to_string_lossy();
        let expected = custom_skill.to_string_lossy();
        assert!(
            resolved.eq_ignore_ascii_case(&expected)
                || resolved.replace('\\', "/") == expected.replace('\\', "/"),
            "resolved={resolved}, expected={expected}"
        );
        fs::remove_dir_all(home).unwrap();
    }
}
