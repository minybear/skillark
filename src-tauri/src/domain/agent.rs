use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AgentKind {
    ClaudeCode,
    Cursor,
    Codex,
    WorkBuddy,
    Custom(String),
}

impl AgentKind {
    pub fn as_contract_value(&self) -> &str {
        match self {
            Self::ClaudeCode => "claude-code",
            Self::Cursor => "cursor",
            Self::Codex => "codex",
            Self::WorkBuddy => "workbuddy",
            Self::Custom(value) => value,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DetectionSignal {
    pub signal_type: String,
    pub matched: bool,
    pub weight: i32,
    pub detail: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentCandidate {
    pub kind: AgentKind,
    pub display_name: String,
    pub confidence: u8,
    pub executable_path: Option<PathBuf>,
    pub global_skill_path: Option<PathBuf>,
    pub writable: Option<bool>,
    pub signals: Vec<DetectionSignal>,
}
