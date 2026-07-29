//! Deployment drivers — the bytes-on-disk half of install / verify / uninstall.
//!
//! The application service owns the transaction boundary and the audit rows;
//! each driver owns a single strategy. [`copy::CopyDriver`] duplicates the skill
//! tree; [`junction::JunctionDriver`] (Windows only) links to the Library
//! snapshot.

pub mod copy;
pub mod junction;

pub use copy::CopyDriver;
pub use junction::JunctionDriver;

#[derive(Debug, thiserror::Error)]
pub enum DeploymentError {
    #[error(transparent)]
    Fs(#[from] crate::adapters::filesystem::FsError),
    #[error("refusing to write to a dangerous target path: {0}")]
    DangerousTarget(String),
    #[error("installed content hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
    #[error("cannot install over an unmanaged or conflicting target without confirmation: {0}")]
    UnsafeConflict(String),
    #[error("target exists as a file, not a directory: {0}")]
    TargetIsFile(String),
    #[error("hash computation failed: {0}")]
    Hash(String),
    #[error("junction links are not supported on this platform")]
    JunctionUnsupported,
    #[error("{0}")]
    Other(String),
}

/// Defense-in-depth: refuse targets that resolve to a filesystem root. The
/// plan service constructs targets under an agent's skill directory, so this
/// only ever trips on a genuinely bad path.
pub(crate) fn target_is_dangerous(target: &std::path::Path) -> bool {
    // Prefer the target itself; fall back to its parent when it doesn't exist yet.
    let probe = target
        .canonicalize()
        .ok()
        .or_else(|| target.parent().and_then(|p| p.canonicalize().ok()));
    match probe {
        // A path with no parent is a drive/filesystem root.
        Some(c) => c.parent().is_none(),
        None => false,
    }
}

/// Probe the nearest existing directory that would accept a new or replacement
/// target. The immediate parent may itself be a valid, not-yet-created path.
pub(super) fn writable_for_install(target: &std::path::Path) -> bool {
    let mut dir = if target.is_dir() {
        target.to_path_buf()
    } else {
        target
            .parent()
            .map(std::path::Path::to_path_buf)
            .unwrap_or_else(|| std::path::PathBuf::from("."))
    };
    while !dir.is_dir() {
        if !dir.pop() {
            return false;
        }
    }

    let marker = dir.join(format!(".skillark-write-probe-{}", std::process::id()));
    match std::fs::File::create(&marker) {
        Ok(_) => {
            let _ = std::fs::remove_file(&marker);
            true
        }
        Err(_) => false,
    }
}
