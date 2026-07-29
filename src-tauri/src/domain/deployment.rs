//! Deployment domain models.
//!
//! Two layers of types live here:
//!
//! - **Plan-time**: [`DeploymentTarget`] / [`DeploymentPlan`] describe what an
//!   install *intends* to do. They are computed up front, shown to the user, and
//!   never touch the filesystem.
//! - **Record-time**: [`DeploymentRecord`] is the persisted fact that "skill
//!   version V was installed at path P for agent A in workspace W". Drivers and
//!   repositories round-trip it.
//!
//! [`VerifyResult`] is deliberately **not** persisted (ADR-011): verification is
//! a read-only computation whose drift reason is reported but never stored as
//! new deployment state.

use std::path::PathBuf;

use super::{operation::OperationId, workspace::WorkspaceId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InstallMode {
    Copy,
    Junction,
}

impl InstallMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Copy => "copy",
            Self::Junction => "junction",
        }
    }
}

impl std::str::FromStr for InstallMode {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "copy" => Ok(Self::Copy),
            "junction" => Ok(Self::Junction),
            other => Err(format!("unknown install mode: {other}")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConflictKind {
    None,
    ManagedSame,
    ManagedOutdated,
    ManagedModified,
    UnmanagedSkill,
    UnmanagedDirectory,
    FileConflict,
    PermissionDenied,
}

impl ConflictKind {
    /// `true` when installing over this conflict is safe without user sign-off.
    pub fn is_safe_to_overwrite(self) -> bool {
        matches!(self, Self::None | Self::ManagedSame)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::ManagedSame => "managed_same",
            Self::ManagedOutdated => "managed_outdated",
            Self::ManagedModified => "managed_modified",
            Self::UnmanagedSkill => "unmanaged_skill",
            Self::UnmanagedDirectory => "unmanaged_directory",
            Self::FileConflict => "file_conflict",
            Self::PermissionDenied => "permission_denied",
        }
    }
}

impl std::str::FromStr for ConflictKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "none" => Ok(Self::None),
            "managed_same" => Ok(Self::ManagedSame),
            "managed_outdated" => Ok(Self::ManagedOutdated),
            "managed_modified" => Ok(Self::ManagedModified),
            "unmanaged_skill" => Ok(Self::UnmanagedSkill),
            "unmanaged_directory" => Ok(Self::UnmanagedDirectory),
            "file_conflict" => Ok(Self::FileConflict),
            "permission_denied" => Ok(Self::PermissionDenied),
            other => Err(format!("unknown conflict kind: {other}")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeploymentStatus {
    Planned,
    Installing,
    Synced,
    Outdated,
    Modified,
    Missing,
    Failed,
    Uninstalled,
}

impl DeploymentStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Installing => "installing",
            Self::Synced => "synced",
            Self::Outdated => "outdated",
            Self::Modified => "modified",
            Self::Missing => "missing",
            Self::Failed => "failed",
            Self::Uninstalled => "uninstalled",
        }
    }
}

impl std::str::FromStr for DeploymentStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "planned" => Ok(Self::Planned),
            "installing" => Ok(Self::Installing),
            "synced" => Ok(Self::Synced),
            "outdated" => Ok(Self::Outdated),
            "modified" => Ok(Self::Modified),
            "missing" => Ok(Self::Missing),
            "failed" => Ok(Self::Failed),
            "uninstalled" => Ok(Self::Uninstalled),
            other => Err(format!("unknown deployment status: {other}")),
        }
    }
}

/// A single target as it appears in a read-only [`DeploymentPlan`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeploymentTarget {
    pub agent_id: String,
    pub workspace_id: WorkspaceId,
    pub target_path: PathBuf,
    pub mode: InstallMode,
    pub conflict: ConflictKind,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeploymentPlan {
    pub operation_id: OperationId,
    pub skill_version_id: String,
    pub targets: Vec<DeploymentTarget>,
    pub requires_confirmation: bool,
    pub warnings: Vec<String>,
}

impl DeploymentPlan {
    /// A plan needs user sign-off when any single target lands on a conflict
    /// that is not safe to overwrite.
    pub fn requires_confirmation(targets: &[DeploymentTarget]) -> bool {
        targets.iter().any(|t| !t.conflict.is_safe_to_overwrite())
    }
}

/// The persisted fact that a skill version is installed at a target path.
///
/// Mirrors the `deployments` table. `deployed_hash` is the hash recorded at
/// install time; verification compares the live target against it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeploymentRecord {
    pub id: String,
    pub skill_version_id: String,
    pub agent_id: String,
    pub workspace_id: String,
    pub operation_id: Option<String>,
    pub target_path: PathBuf,
    pub install_mode: InstallMode,
    pub status: DeploymentStatus,
    pub deployed_hash: Option<String>,
    pub source_path_at_install: PathBuf,
    pub installed_at: Option<String>,
    pub last_verified_at: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Why a verified target drifted from its recorded state. Reported but never
/// persisted (ADR-011).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DriftReason {
    /// Target matches the recorded deployment.
    None,
    /// Target hash differs from deployed_hash but equals the current Library
    /// version — SkillArk advanced, the target did not.
    LibraryVersionChanged,
    /// Target hash differs from deployed_hash and is not a known Library version.
    UserModified,
    /// Junction points somewhere other than the Library snapshot.
    LinkRetargeted,
    /// Junction exists but its target is gone.
    LinkBroken,
    /// Target path no longer exists.
    TargetMissing,
    /// Verification could not complete (read error, etc.).
    Error(String),
}

/// Read-only result of verifying a [`DeploymentRecord`] against the live
/// filesystem. Not persisted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifyResult {
    pub status: DeploymentStatus,
    pub reason: DriftReason,
    pub observed_hash: Option<String>,
    pub warnings: Vec<String>,
}

impl VerifyResult {
    pub fn synced() -> Self {
        Self {
            status: DeploymentStatus::Synced,
            reason: DriftReason::None,
            observed_hash: None,
            warnings: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn install_mode_round_trips() {
        for mode in [InstallMode::Copy, InstallMode::Junction] {
            let s = mode.as_str();
            assert_eq!(s.parse::<InstallMode>().unwrap(), mode);
        }
        assert!("bogus".parse::<InstallMode>().is_err());
    }

    #[test]
    fn conflict_kind_round_trips_all_variants() {
        let all = [
            ConflictKind::None,
            ConflictKind::ManagedSame,
            ConflictKind::ManagedOutdated,
            ConflictKind::ManagedModified,
            ConflictKind::UnmanagedSkill,
            ConflictKind::UnmanagedDirectory,
            ConflictKind::FileConflict,
            ConflictKind::PermissionDenied,
        ];
        for kind in all {
            let s = kind.as_str();
            assert_eq!(s.parse::<ConflictKind>().unwrap(), kind);
        }
    }

    #[test]
    fn only_none_and_managed_same_are_safe_to_overwrite() {
        let safe = [ConflictKind::None, ConflictKind::ManagedSame];
        let unsafe_ = [
            ConflictKind::ManagedOutdated,
            ConflictKind::ManagedModified,
            ConflictKind::UnmanagedSkill,
            ConflictKind::UnmanagedDirectory,
            ConflictKind::FileConflict,
            ConflictKind::PermissionDenied,
        ];
        assert!(safe.iter().all(|c| c.is_safe_to_overwrite()));
        assert!(unsafe_.iter().all(|c| !c.is_safe_to_overwrite()));
    }

    #[test]
    fn deployment_status_round_trips() {
        for status in [
            DeploymentStatus::Planned,
            DeploymentStatus::Installing,
            DeploymentStatus::Synced,
            DeploymentStatus::Outdated,
            DeploymentStatus::Modified,
            DeploymentStatus::Missing,
            DeploymentStatus::Failed,
            DeploymentStatus::Uninstalled,
        ] {
            let s = status.as_str();
            assert_eq!(s.parse::<DeploymentStatus>().unwrap(), status);
        }
    }

    #[test]
    fn requires_confirmation_is_true_when_any_target_unsafe() {
        use std::path::PathBuf;
        use super::super::workspace::WorkspaceId;

        let safe_target = DeploymentTarget {
            agent_id: "codex".to_owned(),
            workspace_id: WorkspaceId("global-default".to_owned()),
            target_path: PathBuf::from("/t/a"),
            mode: InstallMode::Copy,
            conflict: ConflictKind::None,
            warnings: vec![],
        };
        let mut unsafe_target = safe_target.clone();
        unsafe_target.target_path = PathBuf::from("/t/b");
        unsafe_target.conflict = ConflictKind::UnmanagedDirectory;

        assert!(!DeploymentPlan::requires_confirmation(std::slice::from_ref(&safe_target)));
        assert!(DeploymentPlan::requires_confirmation(&[
            safe_target,
            unsafe_target
        ]));
    }
}
