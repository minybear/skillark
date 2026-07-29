use serde::{Deserialize, Serialize};

use crate::domain::{
    agent::{AgentCandidate, DetectionSignal},
    deployment::{DeploymentRecord, InstallMode},
    operation::Operation,
    skill::Skill,
};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectionSignalDto {
    #[serde(rename = "type")]
    pub signal_type: String,
    pub matched: bool,
    pub weight: i32,
    pub detail: Option<String>,
}

impl From<&DetectionSignal> for DetectionSignalDto {
    fn from(value: &DetectionSignal) -> Self {
        Self {
            signal_type: value.signal_type.clone(),
            matched: value.matched,
            weight: value.weight,
            detail: value.detail.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCandidateDto {
    pub agent_type: String,
    pub display_name: String,
    pub confidence: u8,
    pub executable_path: Option<String>,
    pub global_skill_path: Option<String>,
    pub writable: Option<bool>,
    pub signals: Vec<DetectionSignalDto>,
}

impl From<&AgentCandidate> for AgentCandidateDto {
    fn from(value: &AgentCandidate) -> Self {
        Self {
            agent_type: value.kind.as_contract_value().to_owned(),
            display_name: value.display_name.clone(),
            confidence: value.confidence,
            executable_path: value
                .executable_path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
            global_skill_path: value
                .global_skill_path
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
            writable: value.writable,
            signals: value.signals.iter().map(DetectionSignalDto::from).collect(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum InstallModeDto {
    Copy,
    Junction,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictKindDto {
    None,
    ManagedSame,
    ManagedOutdated,
    ManagedModified,
    UnmanagedSkill,
    UnmanagedDirectory,
    FileConflict,
    PermissionDenied,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentTargetDto {
    pub agent_id: String,
    pub workspace_id: String,
    pub target_path: String,
    pub mode: InstallModeDto,
    pub conflict: ConflictKindDto,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentPlanDto {
    pub operation_id: String,
    pub skill_version_id: String,
    pub targets: Vec<DeploymentTargetDto>,
    pub requires_confirmation: bool,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillFileDto {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillManifestDto {
    pub name: String,
    pub description: String,
    pub format: String,
    pub files: Vec<SkillFileDto>,
    pub content_hash: String,
    pub metadata: serde_json::Value,
    pub warnings: Vec<String>,
}

// ===== v0.1 Library / Deploy DTOs =====

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillDto {
    pub id: String,
    pub canonical_name: String,
    pub display_name: String,
    pub description: String,
    pub format: String,
    pub library_path: String,
    pub status: String,
    pub current_version_id: Option<String>,
    pub content_hash: Option<String>,
    pub version_label: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl SkillDto {
    /// Build a list-item DTO from a skill plus its current version (if any).
    pub fn from_skill(skill: &Skill, hash: Option<String>, label: Option<String>) -> Self {
        Self {
            id: skill.id.to_string(),
            canonical_name: skill.canonical_name.clone(),
            display_name: skill.display_name.clone(),
            description: skill.description.clone(),
            format: skill.format.clone(),
            library_path: skill.library_path.clone(),
            status: skill.status.to_string(),
            current_version_id: None,
            content_hash: hash,
            version_label: label,
            created_at: skill.created_at.clone(),
            updated_at: skill.updated_at.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillVersionDto {
    pub id: String,
    pub content_hash: String,
    pub version_label: Option<String>,
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentDto {
    pub id: String,
    pub skill_version_id: String,
    pub agent_id: String,
    pub workspace_id: String,
    pub target_path: String,
    pub mode: String,
    pub status: String,
    pub deployed_hash: Option<String>,
    pub installed_at: Option<String>,
    pub last_verified_at: Option<String>,
    pub error_message: Option<String>,
}

impl From<&DeploymentRecord> for DeploymentDto {
    fn from(record: &DeploymentRecord) -> Self {
        Self {
            id: record.id.clone(),
            skill_version_id: record.skill_version_id.clone(),
            agent_id: record.agent_id.clone(),
            workspace_id: record.workspace_id.clone(),
            target_path: record.target_path.to_string_lossy().into_owned(),
            mode: record.install_mode.as_str().to_owned(),
            status: record.status.as_str().to_owned(),
            deployed_hash: record.deployed_hash.clone(),
            installed_at: record.installed_at.clone(),
            last_verified_at: record.last_verified_at.clone(),
            error_message: record.error_message.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationDto {
    pub id: String,
    pub operation_type: String,
    pub status: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub error_message: Option<String>,
    pub result_json: Option<String>,
}

impl From<&Operation> for OperationDto {
    fn from(op: &Operation) -> Self {
        Self {
            id: op.id.clone(),
            operation_type: op.operation_type.as_str().to_owned(),
            status: op.status.as_str().to_owned(),
            started_at: op.started_at.clone(),
            completed_at: op.completed_at.clone(),
            error_message: op.error_message.clone(),
            result_json: op.result_json.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanTargetSpecDto {
    pub agent_id: String,
    pub workspace_id: String,
    pub target_path: String,
    pub mode: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanRequestDto {
    pub skill_version_id: String,
    pub targets: Vec<PlanTargetSpecDto>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceDto {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub root_path: Option<String>,
    pub status: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanTargetDto {
    pub agent_id: String,
    pub workspace_id: String,
    pub target_path: String,
    pub mode: String,
    pub conflict: String,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanDto {
    pub operation_id: String,
    pub skill_version_id: String,
    pub requires_confirmation: bool,
    pub warnings: Vec<String>,
    pub targets: Vec<PlanTargetDto>,
}

impl PlanDto {
    /// Reconstruct the domain plan from the wire DTO. Unknown enum strings fall
    /// back to safe defaults (copy / none) so a malformed payload cannot panic.
    pub fn to_plan(&self) -> crate::domain::deployment::DeploymentPlan {
        use std::str::FromStr;
        let targets = self
            .targets
            .iter()
            .map(|t| crate::domain::deployment::DeploymentTarget {
                agent_id: t.agent_id.clone(),
                workspace_id: crate::domain::workspace::WorkspaceId(t.workspace_id.clone()),
                target_path: std::path::PathBuf::from(&t.target_path),
                mode: InstallMode::from_str(&t.mode).unwrap_or(InstallMode::Copy),
                conflict: crate::domain::deployment::ConflictKind::from_str(&t.conflict)
                    .unwrap_or(crate::domain::deployment::ConflictKind::None),
                warnings: t.warnings.clone(),
            })
            .collect();
        crate::domain::deployment::DeploymentPlan {
            operation_id: crate::domain::operation::OperationId(self.operation_id.clone()),
            skill_version_id: self.skill_version_id.clone(),
            targets,
            requires_confirmation: self.requires_confirmation,
            warnings: self.warnings.clone(),
        }
    }
}

