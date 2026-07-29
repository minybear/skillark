//! Uninstall a deployment through its driver and audit the action.

use crate::{
    application::plan_deployment::driver_for,
    application::state::AppState,
    domain::{deployment::DeploymentStatus, operation::OperationType},
};
use crate::domain::operation::OperationStatus;

pub struct UninstallDeploymentService<'a> {
    pub state: &'a AppState,
}

impl<'a> UninstallDeploymentService<'a> {
    pub fn new(state: &'a AppState) -> Self {
        Self { state }
    }

    pub async fn uninstall(
        &self,
        deployment_id: &str,
        force: bool,
    ) -> Result<UninstallOutcome, String> {
        let dep_repo = self.state.deployments();
        let op_repo = self.state.operations();

        let record = dep_repo
            .get(deployment_id)
            .await?
            .ok_or_else(|| format!("deployment {deployment_id} not found"))?;

        let operation_id = uuid::Uuid::new_v4().to_string();
        op_repo
            .create(
                &operation_id,
                OperationType::Uninstall,
                &format!(r#"{{"deploymentId":"{deployment_id}","force":{force}}}"#),
            )
            .await?;

        if let Err(message) =
            crate::domain::path_safety::validate_deployment_target(&record.target_path)
        {
            let _ = op_repo
                .complete(
                    &operation_id,
                    OperationStatus::Failed,
                    None,
                    Some(&message),
                )
                .await;
            return Err(message);
        }

        let driver = driver_for(record.install_mode);
        let result = match driver.uninstall(&record, force) {
            Ok(r) => r,
            Err(err) => {
                let msg = err.to_string();
                let _ = op_repo
                    .complete(&operation_id, OperationStatus::Failed, None, Some(&msg))
                    .await;
                return Err(msg);
            }
        };

        // If the target was removed, retire the deployment row.
        if result.removed_target {
            let _ = dep_repo
                .set_status(deployment_id, DeploymentStatus::Uninstalled, None)
                .await;
        } else if result.status == DeploymentStatus::Modified {
            let _ = dep_repo
                .set_status(deployment_id, DeploymentStatus::Modified, Some(&result.message))
                .await;
        }

        let status = if result.removed_target {
            OperationStatus::Succeeded
        } else {
            OperationStatus::Failed
        };
        let _ = op_repo
            .complete(&operation_id, status, None, None)
            .await;

        Ok(UninstallOutcome {
            deployment_id: deployment_id.to_owned(),
            removed_target: result.removed_target,
            status: result.status.as_str().to_owned(),
            message: result.message,
            operation_id,
        })
    }
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UninstallOutcome {
    pub deployment_id: String,
    pub removed_target: bool,
    pub status: String,
    pub message: String,
    pub operation_id: String,
}
