//! Operation audit domain models.
//!
//! An [`Operation`] is the durable record of a single write action (import,
//! install batch, uninstall, verify run, repair). The compensating-transaction
//! flow (ARCHITECTURE §7) creates a row with status `running` up front, then
//! advances it to `succeeded` / `failed` once the filesystem work settles.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OperationType {
    Import,
    Install,
    Uninstall,
    Verify,
    Repair,
}

impl OperationType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Import => "import",
            Self::Install => "install",
            Self::Uninstall => "uninstall",
            Self::Verify => "verify",
            Self::Repair => "repair",
        }
    }
}

impl std::str::FromStr for OperationType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "import" => Ok(Self::Import),
            "install" => Ok(Self::Install),
            "uninstall" => Ok(Self::Uninstall),
            "verify" => Ok(Self::Verify),
            "repair" => Ok(Self::Repair),
            other => Err(format!("unknown operation type: {other}")),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OperationStatus {
    Planned,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

impl OperationStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

impl std::str::FromStr for OperationStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "planned" => Ok(Self::Planned),
            "running" => Ok(Self::Running),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            other => Err(format!("unknown operation status: {other}")),
        }
    }
}

/// Newtype id for an operation. Surfaced to the UI and referenced by
/// `deployments.operation_id`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct OperationId(pub String);

impl OperationId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A durable audit row for one write action.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Operation {
    pub id: String,
    pub operation_type: OperationType,
    pub status: OperationStatus,
    /// Structured plan (e.g. the serialized [`DeploymentPlan`](super::deployment::DeploymentPlan)).
    pub plan_json: String,
    /// Structured outcome — per-target results for installs, counts for imports.
    pub result_json: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn operation_type_round_trips() {
        for t in [
            OperationType::Import,
            OperationType::Install,
            OperationType::Uninstall,
            OperationType::Verify,
            OperationType::Repair,
        ] {
            assert_eq!(t.as_str().parse::<OperationType>().unwrap(), t);
        }
    }

    #[test]
    fn operation_status_round_trips() {
        for s in [
            OperationStatus::Planned,
            OperationStatus::Running,
            OperationStatus::Succeeded,
            OperationStatus::Failed,
            OperationStatus::Cancelled,
        ] {
            assert_eq!(s.as_str().parse::<OperationStatus>().unwrap(), s);
        }
    }
}
