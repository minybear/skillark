use std::{
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

const OVERRIDES_FILE_NAME: &str = "agent_overrides.json";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentOverride {
    #[serde(rename = "agentType")]
    pub agent_type: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "cliName")]
    pub cli_name: Option<String>,
    #[serde(rename = "configDir")]
    pub config_dir: Option<String>,
    #[serde(rename = "skillDir")]
    pub skill_dir: Option<String>,
    #[serde(rename = "skillPathOverride")]
    pub skill_path_override: Option<String>,
    #[serde(rename = "isCustom")]
    pub is_custom: bool,
}

fn overrides_dir() -> Result<PathBuf, String> {
    let dir = super::state::skillark_dir()?;
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create overrides directory: {e}"))?;
    Ok(dir)
}

fn overrides_path() -> Result<PathBuf, String> {
    Ok(overrides_dir()?.join(OVERRIDES_FILE_NAME))
}

pub fn load_overrides() -> Vec<AgentOverride> {
    match overrides_path() {
        Ok(path) => read_overrides(&path).unwrap_or_default(),
        Err(_) => Vec::new(),
    }
}

fn read_overrides(path: &Path) -> Result<Vec<AgentOverride>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read overrides file: {e}"))?;
    if content.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse overrides JSON: {e}"))
}

pub fn save_override(request: AgentOverride) -> Result<(), String> {
    let path = overrides_path()?;
    let mut existing = read_overrides(&path).unwrap_or_default();
    if let Some(slot) = existing.iter_mut().find(|o| o.agent_type == request.agent_type) {
        *slot = request;
    } else {
        existing.push(request);
    }
    existing.sort_by(|a, b| a.agent_type.cmp(&b.agent_type));
    write_overrides(&path, &existing)
}

pub fn delete_override(agent_type: &str) -> Result<(), String> {
    let path = overrides_path()?;
    let mut existing = read_overrides(&path).unwrap_or_default();
    let before = existing.len();
    existing.retain(|o| o.agent_type != agent_type);
    if existing.len() == before {
        return Ok(());
    }
    write_overrides(&path, &existing)
}

fn write_overrides(path: &Path, overrides: &[AgentOverride]) -> Result<(), String> {
    let content = serde_json::to_string_pretty(overrides)
        .map_err(|e| format!("Failed to serialize overrides: {e}"))?;
    fs::write(path, content).map_err(|e| format!("Failed to write overrides file: {e}"))
}

#[cfg(test)]
mod tests {
    use super::{delete_override, load_overrides, save_override, AgentOverride};

    fn sample(type_name: &str) -> AgentOverride {
        AgentOverride {
            agent_type: type_name.to_owned(),
            display_name: type_name.to_owned(),
            cli_name: None,
            config_dir: None,
            skill_dir: None,
            skill_path_override: None,
            is_custom: true,
        }
    }

    #[test]
    fn save_load_delete_roundtrip() {
        let test_agent_type =
            format!("test-override-{}", std::process::id());
        // Save
        save_override(sample(&test_agent_type)).expect("save should succeed");
        // Load
        let loaded = load_overrides();
        assert!(loaded.iter().any(|o| o.agent_type == test_agent_type));
        // Delete
        delete_override(&test_agent_type).expect("delete should succeed");
        let loaded2 = load_overrides();
        assert!(!loaded2.iter().any(|o| o.agent_type == test_agent_type));
    }
}
