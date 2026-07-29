//! Execute a [`DeploymentPlan`]: install each target through its driver, record
//! per-target results, and audit the whole batch as one Operation.
//!
//! A single target failure never hides the others (PRD §5.5, UI-IA §6). The
//! operation status is `succeeded` only when every target installed; otherwise
//! `failed`, and each target's outcome (success or error) is persisted in
//! `operations.result_json`.

use serde::Serialize;

use crate::{
    application::{plan_deployment::driver_for, state::AppState},
    domain::{
        deployment::{DeploymentPlan, DeploymentRecord, DeploymentStatus},
        operation::{OperationStatus, OperationType},
    },
    ports::InstallRequest,
};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetOutcome {
    pub agent_id: String,
    pub workspace_id: String,
    pub target_path: String,
    pub mode: String,
    pub conflict: String,
    pub ok: bool,
    pub deployed_hash: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionReport {
    pub operation_id: String,
    pub skill_version_id: String,
    pub succeeded: usize,
    pub failed: usize,
    pub outcomes: Vec<TargetOutcome>,
}

pub struct ExecuteDeploymentService<'a> {
    pub state: &'a AppState,
}

impl<'a> ExecuteDeploymentService<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    pub async fn execute(&self, plan: DeploymentPlan) -> Result<ExecutionReport, String> {
        let op_repo = self.state.operations();
        let dep_repo = self.state.deployments();

        // Resolve the version the plan deploys (snapshot path + content hash).
        let version_id = uuid::Uuid::parse_str(&plan.skill_version_id)
            .map_err(|e| format!("skill_version_id: {e}"))?;
        let version = self
            .state
            .skills()
            .get_version(version_id)
            .await?
            .ok_or_else(|| format!("skill version {} not found", plan.skill_version_id))?;
        let library_snapshot_path = std::path::PathBuf::from(&version.library_snapshot_path);
        let library_hash = version.content_hash;

        let operation_id = plan.operation_id.0.clone();
        let plan_json = serialize_plan(&plan)?;
        op_repo
            .create(&operation_id, OperationType::Install, &plan_json)
            .await?;

        let now = chrono::Utc::now().to_rfc3339();
        let mut outcomes: Vec<TargetOutcome> = Vec::with_capacity(plan.targets.len());

        for target in &plan.targets {
            if let Err(error) =
                crate::domain::path_safety::validate_deployment_target(&target.target_path)
            {
                outcomes.push(TargetOutcome {
                    agent_id: target.agent_id.clone(),
                    workspace_id: target.workspace_id.0.clone(),
                    target_path: target.target_path.to_string_lossy().into_owned(),
                    mode: target.mode.as_str().to_owned(),
                    conflict: target.conflict.as_str().to_owned(),
                    ok: false,
                    deployed_hash: None,
                    error: Some(error),
                });
                continue;
            }
            let driver = driver_for(target.mode);
            let request = InstallRequest {
                operation_id: operation_id.clone(),
                source: library_snapshot_path.clone(),
                target: target.target_path.clone(),
                expected_hash: library_hash.clone(),
                // Execute runs only after the plan's confirmation gate, so a
                // managed target may be replaced.
                allow_replace_managed: true,
            };

            match driver.install(request) {
                Ok(result) => {
                    // Resolve target.agent_id (an agent_type slug) to the
                    // agents.id UUID before constructing the deployment
                    // record — deployments.agent_id is an FK to agents.id.
                    let agent_id = match self
                        .state
                        .agents()
                        .get_id_by_type(&target.agent_id)
                        .await
                    {
                        Ok(Some(id)) => id.to_string(),
                        Ok(None) => {
                            outcomes.push(TargetOutcome {
                                agent_id: target.agent_id.clone(),
                                workspace_id: target.workspace_id.0.clone(),
                                target_path: target.target_path.to_string_lossy().into_owned(),
                                mode: target.mode.as_str().to_owned(),
                                conflict: target.conflict.as_str().to_owned(),
                                ok: false,
                                deployed_hash: None,
                                error: Some(format!(
                                    "agent {} not registered",
                                    target.agent_id
                                )),
                            });
                            continue;
                        }
                        Err(e) => {
                            outcomes.push(TargetOutcome {
                                agent_id: target.agent_id.clone(),
                                workspace_id: target.workspace_id.0.clone(),
                                target_path: target.target_path.to_string_lossy().into_owned(),
                                mode: target.mode.as_str().to_owned(),
                                conflict: target.conflict.as_str().to_owned(),
                                ok: false,
                                deployed_hash: None,
                                error: Some(format!("resolve agent id: {e}")),
                            });
                            continue;
                        }
                    };
                    let deployed_hash = result
                        .deployed_hash
                        .clone()
                        .or_else(|| Some(library_hash.clone()));
                    let record = DeploymentRecord {
                        id: uuid::Uuid::new_v4().to_string(),
                        skill_version_id: plan.skill_version_id.clone(),
                        agent_id,
                        workspace_id: target.workspace_id.0.clone(),
                        operation_id: Some(operation_id.clone()),
                        target_path: target.target_path.clone(),
                        install_mode: target.mode,
                        status: DeploymentStatus::Synced,
                        deployed_hash: deployed_hash.clone(),
                        source_path_at_install: library_snapshot_path.clone(),
                        installed_at: Some(now.clone()),
                        last_verified_at: None,
                        error_message: None,
                        created_at: now.clone(),
                        updated_at: now.clone(),
                    };
                    // Persist; a DB failure here is a per-target error, not fatal.
                    if let Err(e) = dep_repo.upsert_active(&record).await {
                        outcomes.push(TargetOutcome {
                            agent_id: target.agent_id.clone(),
                            workspace_id: target.workspace_id.0.clone(),
                            target_path: target.target_path.to_string_lossy().into_owned(),
                            mode: target.mode.as_str().to_owned(),
                            conflict: target.conflict.as_str().to_owned(),
                            ok: false,
                            deployed_hash: None,
                            error: Some(format!("persist deployment: {e}")),
                        });
                        continue;
                    }
                    outcomes.push(TargetOutcome {
                        agent_id: target.agent_id.clone(),
                        workspace_id: target.workspace_id.0.clone(),
                        target_path: target.target_path.to_string_lossy().into_owned(),
                        mode: target.mode.as_str().to_owned(),
                        conflict: target.conflict.as_str().to_owned(),
                        ok: true,
                        deployed_hash,
                        error: None,
                    });
                }
                Err(err) => outcomes.push(TargetOutcome {
                    agent_id: target.agent_id.clone(),
                    workspace_id: target.workspace_id.0.clone(),
                    target_path: target.target_path.to_string_lossy().into_owned(),
                    mode: target.mode.as_str().to_owned(),
                    conflict: target.conflict.as_str().to_owned(),
                    ok: false,
                    deployed_hash: None,
                    error: Some(err.to_string()),
                }),
            }
        }

        let succeeded = outcomes.iter().filter(|o| o.ok).count();
        let failed = outcomes.len() - succeeded;
        let status = if failed == 0 {
            OperationStatus::Succeeded
        } else {
            OperationStatus::Failed
        };

        let report = ExecutionReport {
            operation_id: operation_id.clone(),
            skill_version_id: plan.skill_version_id.clone(),
            succeeded,
            failed,
            outcomes: outcomes.clone(),
        };
        let result_json = serde_json::to_string(&report).map_err(|e| e.to_string())?;
        let error_summary = if failed > 0 {
            Some(format!("{failed} of {} targets failed", outcomes.len()))
        } else {
            None
        };
        op_repo
            .complete(&operation_id, status, Some(&result_json), error_summary.as_deref())
            .await?;

        Ok(report)
    }
}

fn serialize_plan(plan: &DeploymentPlan) -> Result<String, String> {
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct PlanTargetJson {
        agent_id: String,
        workspace_id: String,
        target_path: String,
        mode: String,
        conflict: String,
    }
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct PlanJson {
        operation_id: String,
        skill_version_id: String,
        targets: Vec<PlanTargetJson>,
        requires_confirmation: bool,
    }
    let json = PlanJson {
        operation_id: plan.operation_id.0.clone(),
        skill_version_id: plan.skill_version_id.clone(),
        targets: plan
            .targets
            .iter()
            .map(|t| PlanTargetJson {
                agent_id: t.agent_id.clone(),
                workspace_id: t.workspace_id.0.clone(),
                target_path: t.target_path.to_string_lossy().into_owned(),
                mode: t.mode.as_str().to_owned(),
                conflict: t.conflict.as_str().to_owned(),
            })
            .collect(),
        requires_confirmation: plan.requires_confirmation,
    };
    serde_json::to_string(&json).map_err(|e| e.to_string())
}
