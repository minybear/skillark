//! Verify active deployments against the live filesystem.
//!
//! Per ADR-011, verification does not mutate the deployment's persisted status
//! (it stays `synced`); only `last_verified_at` is recorded. The computed drift
//! is returned to the caller for display.

use std::path::Path;

use serde::Serialize;

use crate::{
    application::{plan_deployment::driver_for, state::AppState},
    domain::deployment::{DeploymentStatus, DriftReason},
};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyReportItem {
    pub deployment_id: String,
    pub agent_id: String,
    pub target_path: String,
    pub mode: String,
    pub status: String,
    pub reason: String,
    pub observed_hash: Option<String>,
}

pub struct VerifyDeploymentService<'a> {
    pub state: &'a AppState,
}

impl<'a> VerifyDeploymentService<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    pub async fn verify_all(&self) -> Result<Vec<VerifyReportItem>, String> {
        let dep_repo = self.state.deployments();
        let skill_repo = self.state.skills();
        let deployments = dep_repo.list_active().await?;
        let mut reports = Vec::with_capacity(deployments.len());

        for dep in deployments {
            reports.push(self.verify_one_record(&dep_repo, &skill_repo, dep).await?);
        }
        Ok(reports)
    }

    pub async fn verify_one(&self, deployment_id: &str) -> Result<VerifyReportItem, String> {
        let dep_repo = self.state.deployments();
        let skill_repo = self.state.skills();
        let dep = dep_repo
            .get(deployment_id)
            .await?
            .ok_or_else(|| format!("deployment {deployment_id} not found"))?;
        self.verify_one_record(&dep_repo, &skill_repo, dep).await
    }

    async fn verify_one_record(
        &self,
        dep_repo: &crate::repositories::DeploymentRepository,
        skill_repo: &crate::repositories::SkillRepository,
        dep: crate::domain::deployment::DeploymentRecord,
    ) -> Result<VerifyReportItem, String> {
        let now = chrono::Utc::now().to_rfc3339();

        let version_id = uuid::Uuid::parse_str(&dep.skill_version_id)
            .map_err(|e| format!("version id: {e}"))?;
        let version = match skill_repo.get_version(version_id).await? {
            Some(v) => v,
            None => {
                let _ = dep_repo.mark_verified(&dep.id, &now).await;
                return Ok(VerifyReportItem {
                    deployment_id: dep.id,
                    agent_id: dep.agent_id,
                    target_path: dep.target_path.to_string_lossy().into_owned(),
                    mode: dep.install_mode.as_str().to_owned(),
                    status: DeploymentStatus::Failed.as_str().to_owned(),
                    reason: "skill version no longer in library".to_owned(),
                    observed_hash: None,
                });
            }
        };

        let driver = driver_for(dep.install_mode);
        let result = driver
            .verify(&dep, Path::new(&version.library_snapshot_path), &version.content_hash)
            .map_err(|e| e.to_string())?;

        let _ = dep_repo.mark_verified(&dep.id, &now).await;

        Ok(VerifyReportItem {
            deployment_id: dep.id,
            agent_id: dep.agent_id,
            target_path: dep.target_path.to_string_lossy().into_owned(),
            mode: dep.install_mode.as_str().to_owned(),
            status: result.status.as_str().to_owned(),
            reason: reason_label(&result.reason),
            observed_hash: result.observed_hash,
        })
    }
}

fn reason_label(reason: &DriftReason) -> String {
    match reason {
        DriftReason::None => "none",
        DriftReason::LibraryVersionChanged => "library_version_changed",
        DriftReason::UserModified => "user_modified",
        DriftReason::LinkRetargeted => "link_retargeted",
        DriftReason::LinkBroken => "link_broken",
        DriftReason::TargetMissing => "target_missing",
        DriftReason::Error(_) => "error",
    }
    .to_owned()
}
