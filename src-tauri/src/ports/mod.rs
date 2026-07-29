//! Ports — the seams between pure domain logic and the outside world.
//!
//! Implementations live in `adapters/`. Keeping these as traits means the
//! application services can be tested with fakes and the domain layer never
//! imports `std::fs`, `sqlx`, or `tauri`.

use std::path::{Path, PathBuf};

use crate::domain::{
    agent::{AgentCandidate, AgentKind},
    deployment::{DeploymentRecord, DeploymentStatus, InstallMode, VerifyResult},
    skill_manifest::SkillManifest,
};

// ── Agent detection ──────────────────────────────────────────────────────

pub struct DetectionContext {
    pub home_dir: PathBuf,
    pub app_data: Option<PathBuf>,
    pub local_app_data: Option<PathBuf>,
    pub program_files: Option<PathBuf>,
    pub program_files_x86: Option<PathBuf>,
    pub path_entries: Vec<PathBuf>,
    pub running_processes: Vec<String>,
    pub manual_skill_paths: std::collections::HashMap<String, PathBuf>,
}

pub struct ValidationResult {
    pub valid: bool,
    pub writable: bool,
    pub warnings: Vec<String>,
}

pub trait AgentAdapter: Send + Sync {
    fn kind(&self) -> AgentKind;
    fn display_name(&self) -> String;
    fn detect(&self, context: &DetectionContext) -> Vec<AgentCandidate>;
    fn validate_configuration(&self, candidate: &AgentCandidate) -> ValidationResult;
    fn global_skill_path(&self, candidate: &AgentCandidate) -> Option<PathBuf>;
    fn project_skill_path(
        &self,
        candidate: &AgentCandidate,
        project_root: &Path,
    ) -> Option<PathBuf>;
}

// ── Skill sources ────────────────────────────────────────────────────────

/// A source that can be scanned into a canonical skill tree.
///
/// `LocalDirectorySource` reads a directory in place; `ZipSource` first
/// extracts into a scratch directory and then presents that as the root. In
/// both cases [`SkillSourceAdapter::scan`] returns the parsed manifest and the
/// root path the import service should copy from.
pub trait SkillSourceAdapter: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    /// Parse the source and return the manifest plus the on-disk root to copy.
    fn scan(&self) -> Result<ScannedSource, Self::Error>;
}

#[derive(Clone, Debug)]
pub struct ScannedSource {
    pub manifest: SkillManifest,
    /// Directory whose contents form the skill. Must exist on the local disk.
    pub source_root: PathBuf,
}

// ── Deployment drivers ───────────────────────────────────────────────────

/// Filesystem-level snapshot of an install target, used by the plan service to
/// classify the conflict. The driver produces this; combining it with the
/// persisted deployment record (managed vs unmanaged) is the service's job.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TargetProbe {
    pub exists: bool,
    pub is_file: bool,
    pub is_dir: bool,
    pub writable: bool,
    pub has_skill_md: bool,
    /// Content hash when the target is a directory SkillArk can read, else None.
    pub current_hash: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallRequest {
    pub operation_id: String,
    pub source: PathBuf,
    pub target: PathBuf,
    /// Hash the installed copy must match, computed from the Library snapshot.
    pub expected_hash: String,
    /// When true, the driver may replace a target that SkillArk already manages
    /// without further confirmation (used for re-deploy / managed_outdated).
    pub allow_replace_managed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstallResult {
    /// Hash observed at the target after install (Copy mode). None for Junction.
    pub deployed_hash: Option<String>,
    pub target_path: PathBuf,
}

/// Outcome of an uninstall. Copy mode refuses to delete a user-modified target
/// unless `force` is set; in that case `removed_target` is false and `message`
/// explains the refusal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UninstallResult {
    pub removed_target: bool,
    pub status: DeploymentStatus,
    pub message: String,
}

/// A deployment strategy (Copy or Junction). Each method is the smallest unit
/// the application service orchestrates; the service owns transaction boundaries
/// and audit, the driver owns bytes-on-disk.
pub trait DeploymentDriver: Send + Sync {
    type Error: std::error::Error + Send + Sync + 'static;

    fn mode(&self) -> InstallMode;

    /// Read-only probe of a target path.
    fn probe(&self, target: &Path) -> Result<TargetProbe, Self::Error>;

    /// Materialize the skill at `target`. Must be compensating: on any failure
    /// the previous target (if any) must remain usable, and no half-written
    /// temp directory may be left behind.
    fn install(&self, request: InstallRequest) -> Result<InstallResult, Self::Error>;

    /// Compare the live target against the recorded deployment.
    fn verify(
        &self,
        record: &DeploymentRecord,
        library_snapshot_path: &Path,
        library_hash: &str,
    ) -> Result<VerifyResult, Self::Error>;

    /// Remove the deployment. For Copy, refuses modified targets unless forced.
    fn uninstall(
        &self,
        record: &DeploymentRecord,
        force: bool,
    ) -> Result<UninstallResult, Self::Error>;
}

// ── Cross-cutting ────────────────────────────────────────────────────────

pub trait Clock: Send + Sync {
    fn now_utc(&self) -> String;
}
