//! Build a read-only [`DeploymentPlan`] by classifying each target's conflict.
//!
//! Conflict classification combines a filesystem probe (from the driver) with
//! the persisted active deployment record (managed vs unmanaged). The driver
//! owns bytes-on-disk; this service owns the cross-cutting classification rule.

use std::path::PathBuf;

use uuid::Uuid;

use crate::{
    adapters::deployment::{CopyDriver, DeploymentError, JunctionDriver},
    application::state::AppState,
    domain::{
        deployment::{ConflictKind, DeploymentPlan, DeploymentTarget, InstallMode},
        operation::OperationId,
        workspace::WorkspaceId,
    },
    ports::{DeploymentDriver, TargetProbe},
};
use crate::domain::deployment::DeploymentRecord;

/// Pick the driver implementing a given install mode.
pub fn driver_for(mode: InstallMode) -> Box<dyn DeploymentDriver<Error = DeploymentError>> {
    match mode {
        InstallMode::Copy => Box::new(CopyDriver::new()),
        InstallMode::Junction => Box::new(JunctionDriver::new()),
    }
}

/// Pure conflict classification. See `02-方案设计 §2.2`.
pub fn classify(
    probe: &TargetProbe,
    existing: Option<&DeploymentRecord>,
    library_hash: &str,
) -> ConflictKind {
    if !probe.exists {
        return if probe.writable {
            ConflictKind::None
        } else {
            ConflictKind::PermissionDenied
        };
    }
    if probe.is_file {
        return ConflictKind::FileConflict;
    }
    if !probe.writable {
        return ConflictKind::PermissionDenied;
    }
    match existing {
        Some(record) => {
            let current = probe.current_hash.as_deref();
            match (current, record.deployed_hash.as_deref()) {
                (Some(current), Some(deployed)) if current == deployed => ConflictKind::ManagedSame,
                (Some(current), _) if current == library_hash => ConflictKind::ManagedOutdated,
                _ => ConflictKind::ManagedModified,
            }
        }
        None => {
            if probe.has_skill_md {
                ConflictKind::UnmanagedSkill
            } else {
                ConflictKind::UnmanagedDirectory
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct PlanTargetSpec {
    pub agent_id: String,
    pub workspace_id: WorkspaceId,
    pub target_path: PathBuf,
    pub mode: InstallMode,
}

#[derive(Clone, Debug)]
pub struct PlanRequest {
    pub skill_version_id: String,
    pub targets: Vec<PlanTargetSpec>,
}

pub struct PlanDeploymentService<'a> {
    pub state: &'a AppState,
}

impl<'a> PlanDeploymentService<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    pub async fn build_plan(&self, request: PlanRequest) -> Result<DeploymentPlan, String> {
        // Resolve the version once to get the content hash used for classification.
        let version_id = uuid::Uuid::parse_str(&request.skill_version_id)
            .map_err(|e| format!("skill_version_id: {e}"))?;
        let version = self
            .state
            .skills()
            .get_version(version_id)
            .await?
            .ok_or_else(|| format!("skill version {} not found", request.skill_version_id))?;
        let library_hash = version.content_hash;

        let dep_repo = self.state.deployments();
        let mut targets = Vec::with_capacity(request.targets.len());

        for spec in request.targets {
            crate::domain::path_safety::validate_deployment_target(&spec.target_path)?;
            let driver = driver_for(spec.mode);
            let probe = driver.probe(&spec.target_path).map_err(|e| e.to_string())?;
            let existing = dep_repo.find_active_by_target(&spec.target_path).await?;
            let conflict = classify(&probe, existing.as_ref(), &library_hash);

            targets.push(DeploymentTarget {
                agent_id: spec.agent_id,
                workspace_id: spec.workspace_id,
                target_path: spec.target_path,
                mode: spec.mode,
                conflict,
                warnings: Vec::new(),
            });
        }

        let requires_confirmation = DeploymentPlan::requires_confirmation(&targets);
        Ok(DeploymentPlan {
            operation_id: OperationId::new(Uuid::new_v4().to_string()),
            skill_version_id: request.skill_version_id,
            targets,
            requires_confirmation,
            warnings: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::deployment::{DeploymentStatus};

    fn probe(exists: bool, is_file: bool, writable: bool, has_skill_md: bool, hash: Option<&str>) -> TargetProbe {
        TargetProbe {
            exists,
            is_file,
            is_dir: exists && !is_file,
            writable,
            has_skill_md,
            current_hash: hash.map(str::to_owned),
        }
    }

    fn record_with_deployed(deployed: Option<&str>) -> DeploymentRecord {
        DeploymentRecord {
            id: "r".to_owned(),
            skill_version_id: "sv".to_owned(),
            agent_id: "a".to_owned(),
            workspace_id: "global-default".to_owned(),
            operation_id: None,
            target_path: PathBuf::from("/t"),
            install_mode: InstallMode::Copy,
            status: DeploymentStatus::Synced,
            deployed_hash: deployed.map(str::to_owned),
            source_path_at_install: PathBuf::from("/s"),
            installed_at: None,
            last_verified_at: None,
            error_message: None,
            created_at: "t".to_owned(),
            updated_at: "t".to_owned(),
        }
    }

    const LIB: &str = "library-hash-000";

    #[test]
    fn missing_target_is_none() {
        assert_eq!(classify(&probe(false, false, true, false, None), None, LIB), ConflictKind::None);
    }

    #[test]
    fn missing_target_under_read_only_parent_is_permission_denied() {
        assert_eq!(
            classify(&probe(false, false, false, false, None), None, LIB),
            ConflictKind::PermissionDenied
        );
    }

    #[test]
    fn file_target_is_file_conflict() {
        assert_eq!(
            classify(&probe(true, true, true, false, None), None, LIB),
            ConflictKind::FileConflict
        );
    }

    #[test]
    fn read_only_target_is_permission_denied() {
        assert_eq!(
            classify(&probe(true, false, false, false, None), None, LIB),
            ConflictKind::PermissionDenied
        );
    }

    #[test]
    fn unmanaged_dir_vs_unmanaged_skill() {
        assert_eq!(
            classify(&probe(true, false, true, false, None), None, LIB),
            ConflictKind::UnmanagedDirectory
        );
        assert_eq!(
            classify(&probe(true, false, true, true, None), None, LIB),
            ConflictKind::UnmanagedSkill
        );
    }

    #[test]
    fn managed_same_when_target_matches_deployed() {
        let rec = record_with_deployed(Some("h1"));
        assert_eq!(
            classify(&probe(true, false, true, false, Some("h1")), Some(&rec), LIB),
            ConflictKind::ManagedSame
        );
    }

    #[test]
    fn managed_outdated_when_target_matches_library_but_not_deployed() {
        let rec = record_with_deployed(Some("old"));
        assert_eq!(
            classify(&probe(true, false, true, false, Some(LIB)), Some(&rec), LIB),
            ConflictKind::ManagedOutdated
        );
    }

    #[test]
    fn managed_modified_when_target_differs() {
        let rec = record_with_deployed(Some("h1"));
        assert_eq!(
            classify(&probe(true, false, true, false, Some("edited")), Some(&rec), LIB),
            ConflictKind::ManagedModified
        );
    }
}
