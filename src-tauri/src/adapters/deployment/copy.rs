//! Copy-mode deployment driver.
//!
//! Install is a compensating transaction (DEPLOYMENT-POC §3, ARCHITECTURE §7):
//!
//! 1. `copy_tree(source → tmp)` (sibling of target, same volume)
//! 2. `hash(tmp) == expected_hash` — else discard tmp and fail
//! 3. if target exists, rename it to a sibling `backup`
//! 4. atomic-rename `tmp → target`
//! 5. success → delete `backup`; failure (anywhere after step 3) → restore
//!    `backup → target` and delete `tmp`
//!
//! On success the old target is either gone (was backed up then deleted) or
//! never existed; on failure the previous target is restored. No half-written
//! temp directory survives.

use std::path::Path;

use crate::{
    adapters::{
        deployment::{target_is_dangerous, writable_for_install, DeploymentError},
        filesystem::{
            backup_path_for, copy_tree, force_remove_dir_all, rename_atomic, tmp_path_for,
        },
    },
    domain::{
        content_hash::hash_directory,
        deployment::{DeploymentRecord, DeploymentStatus, DriftReason, InstallMode, VerifyResult},
    },
    ports::{DeploymentDriver, InstallRequest, InstallResult, TargetProbe, UninstallResult},
};

#[derive(Default, Debug, Clone, Copy)]
pub struct CopyDriver;

impl CopyDriver {
    pub fn new() -> Self {
        Self
    }
}

impl DeploymentDriver for CopyDriver {
    type Error = DeploymentError;

    fn mode(&self) -> InstallMode {
        InstallMode::Copy
    }

    fn probe(&self, target: &Path) -> Result<TargetProbe, Self::Error> {
        let meta = match std::fs::symlink_metadata(target) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(TargetProbe {
                    exists: false,
                    is_file: false,
                    is_dir: false,
                    writable: writable_for_install(target),
                    has_skill_md: false,
                    current_hash: None,
                });
            }
            Err(e) => return Err(DeploymentError::Other(format!("probe {}: {e}", target.display()))),
        };

        let is_dir = meta.is_dir();
        let is_file = meta.is_file();
        let has_skill_md = is_dir && target.join("SKILL.md").is_file();
        let current_hash = if is_dir {
            hash_directory(target).map(Some).unwrap_or(None)
        } else {
            None
        };

        Ok(TargetProbe {
            exists: true,
            is_file,
            is_dir,
            writable: writable_for_install(target),
            has_skill_md,
            current_hash,
        })
    }

    fn install(&self, request: InstallRequest) -> Result<InstallResult, Self::Error> {
        let source = request.source.as_path();
        let target = request.target.as_path();

        if target_is_dangerous(target) {
            return Err(DeploymentError::DangerousTarget(target.display().to_string()));
        }
        if source == target {
            return Err(DeploymentError::Other(
                "source and target must not be the same path".to_owned(),
            ));
        }

        let tmp = tmp_path_for(target, &request.operation_id);
        let backup = backup_path_for(target, &request.operation_id);
        // Clear any stale tmp/backup left by a previously crashed run.
        let _ = force_remove_dir_all(&tmp);
        let _ = force_remove_dir_all(&backup);

        // `install_inner` restores the previous target on promotion failure;
        // we additionally guarantee the temp directory never survives an error.
        let result = install_inner(source, target, &tmp, &backup, &request.expected_hash);
        if result.is_err() {
            let _ = force_remove_dir_all(&tmp);
        }
        result.map(|deployed_hash| InstallResult {
            deployed_hash,
            target_path: request.target.clone(),
        })
    }

    fn verify(
        &self,
        record: &DeploymentRecord,
        _library_snapshot_path: &Path,
        library_hash: &str,
    ) -> Result<VerifyResult, Self::Error> {
        let target = record.target_path.as_path();

        let meta = match std::fs::symlink_metadata(target) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(VerifyResult {
                    status: DeploymentStatus::Missing,
                    reason: DriftReason::TargetMissing,
                    observed_hash: None,
                    warnings: vec![],
                });
            }
            Err(e) => {
                return Ok(VerifyResult {
                    status: DeploymentStatus::Failed,
                    reason: DriftReason::Error(e.to_string()),
                    observed_hash: None,
                    warnings: vec![],
                });
            }
        };

        if !meta.is_dir() {
            return Ok(VerifyResult {
                status: DeploymentStatus::Failed,
                reason: DriftReason::Error("target is not a directory".to_owned()),
                observed_hash: None,
                warnings: vec![],
            });
        }

        let observed = hash_directory(target).map_err(DeploymentError::Hash)?;
        let deployed = record.deployed_hash.as_deref();

        let (status, reason) = match deployed {
            Some(deployed) if deployed == observed => {
                if deployed == library_hash {
                    (DeploymentStatus::Synced, DriftReason::None)
                } else {
                    // Target matches what we installed, but the Library moved on.
                    (DeploymentStatus::Outdated, DriftReason::LibraryVersionChanged)
                }
            }
            _ => {
                if observed == library_hash {
                    // Target was advanced to the current Library version out of band.
                    (
                        DeploymentStatus::Synced,
                        DriftReason::LibraryVersionChanged,
                    )
                } else {
                    (DeploymentStatus::Modified, DriftReason::UserModified)
                }
            }
        };

        Ok(VerifyResult {
            status,
            reason,
            observed_hash: Some(observed),
            warnings: vec![],
        })
    }

    fn uninstall(
        &self,
        record: &DeploymentRecord,
        force: bool,
    ) -> Result<UninstallResult, Self::Error> {
        let target = record.target_path.as_path();
        if target_is_dangerous(target) {
            return Err(DeploymentError::DangerousTarget(target.display().to_string()));
        }

        if std::fs::symlink_metadata(target).is_err() {
            return Ok(UninstallResult {
                removed_target: false,
                status: DeploymentStatus::Uninstalled,
                message: "target is already absent".to_owned(),
            });
        }

        // Refuse to silently delete a copy the user has edited.
        let modified = match (hash_directory(target), record.deployed_hash.as_deref()) {
            (Ok(current), Some(deployed)) => current != deployed,
            _ => false,
        };
        if modified && !force {
            return Ok(UninstallResult {
                removed_target: false,
                status: DeploymentStatus::Modified,
                message: "target has been modified since install; force-remove or re-import it"
                    .to_owned(),
            });
        }

        force_remove_dir_all(target)?;
        Ok(UninstallResult {
            removed_target: true,
            status: DeploymentStatus::Uninstalled,
            message: "target removed".to_owned(),
        })
    }
}

/// Core install steps, separated so the caller can guarantee tmp cleanup on
/// every error path. On promotion failure the previous target is restored from
/// `backup` before returning the error.
fn install_inner(
    source: &Path,
    target: &Path,
    tmp: &Path,
    backup: &Path,
    expected_hash: &str,
) -> Result<Option<String>, DeploymentError> {
    // 1. Copy into the sibling temp directory.
    copy_tree(source, tmp)?;

    // 2. Verify the copy hashes as expected before touching the live target.
    let actual = hash_directory(tmp).map_err(DeploymentError::Hash)?;
    if actual != expected_hash {
        return Err(DeploymentError::HashMismatch {
            expected: expected_hash.to_owned(),
            actual,
        });
    }

    // 3. Push the existing target aside (if any).
    let mut backed_up = false;
    if std::fs::symlink_metadata(target).is_ok() {
        rename_atomic(target, backup)?;
        backed_up = true;
    }

    // 4. Promote tmp → target. On failure, restore the previous target.
    if let Err(err) = rename_atomic(tmp, target) {
        if backed_up {
            let _ = rename_atomic(backup, target);
        }
        return Err(err.into());
    }

    // 5. Success — drop the backup.
    if backed_up {
        let _ = force_remove_dir_all(backup);
    }
    Ok(Some(actual))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::deployment::InstallMode;
    use crate::ports::DeploymentDriver;
    use std::path::PathBuf;

    fn unique_tmp(sub: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "skillark-copy-{}-{}-{sub}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn write(path: &Path, body: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, body).unwrap();
    }

    fn source_skill(dir: &Path, body: &str) -> String {
        write(&dir.join("SKILL.md"), body);
        write(&dir.join("scripts/run.sh"), "echo");
        hash_directory(dir).unwrap()
    }

    fn make_record(target: PathBuf, deployed_hash: Option<String>) -> DeploymentRecord {
        DeploymentRecord {
            id: "rec".to_owned(),
            skill_version_id: "sv".to_owned(),
            agent_id: "codex".to_owned(),
            workspace_id: "global-default".to_owned(),
            operation_id: Some("op".to_owned()),
            target_path: target,
            install_mode: InstallMode::Copy,
            status: DeploymentStatus::Synced,
            deployed_hash,
            source_path_at_install: PathBuf::from("/vault/s"),
            installed_at: Some("t".to_owned()),
            last_verified_at: None,
            error_message: None,
            created_at: "t".to_owned(),
            updated_at: "t".to_owned(),
        }
    }

    fn req(op: &str, source: &Path, target: &Path, hash: &str) -> InstallRequest {
        InstallRequest {
            operation_id: op.to_owned(),
            source: source.to_path_buf(),
            target: target.to_path_buf(),
            expected_hash: hash.to_owned(),
            allow_replace_managed: false,
        }
    }

    // ── probe / conflict inputs ──────────────────────────────────────────

    #[test]
    fn probe_reports_none_when_target_missing() {
        let driver = CopyDriver::new();
        let p = unique_tmp("none").join("absent");
        let probe = driver.probe(&p).unwrap();
        assert!(!probe.exists);
        assert_eq!(probe.current_hash, None);
    }

    #[test]
    fn probe_uses_nearest_existing_ancestor_for_nested_missing_target() {
        let driver = CopyDriver::new();
        let p = unique_tmp("nested-missing").join("one/two/three");
        let probe = driver.probe(&p).unwrap();
        assert!(!probe.exists);
        assert!(probe.writable);
    }

    #[test]
    fn probe_classifies_file_conflict_inputs() {
        let driver = CopyDriver::new();
        let dir = unique_tmp("file");
        let file = dir.join("i-am-a-file");
        write(&file, "x");
        let probe = driver.probe(&file).unwrap();
        assert!(probe.exists && probe.is_file && !probe.is_dir);
    }

    #[test]
    fn probe_detects_unmanaged_skill_and_hash() {
        let driver = CopyDriver::new();
        let dir = unique_tmp("unmanaged");
        source_skill(&dir, "---\nname: a\nversion: 1\n---\n");
        let probe = driver.probe(&dir).unwrap();
        assert!(probe.is_dir);
        assert!(probe.has_skill_md);
        assert!(probe.current_hash.is_some());
    }

    // ── install ──────────────────────────────────────────────────────────

    #[test]
    fn install_into_empty_target_then_synced() {
        let driver = CopyDriver::new();
        let root = unique_tmp("ok");
        let src = root.join("src");
        std::fs::create_dir_all(&src).unwrap();
        let hash = source_skill(&src, "---\nname: a\nversion: 1\n---\n");
        let target = root.join("agent/skills/a");

        let res = driver
            .install(req("op1", &src, &target, &hash))
            .expect("install");
        assert_eq!(res.target_path, target);
        assert_eq!(res.deployed_hash.as_deref(), Some(hash.as_str()));
        assert!(target.join("SKILL.md").is_file());
        // No leftover tmp/backup.
        assert!(std::fs::read_dir(target.parent().unwrap())
            .unwrap()
            .all(|e| !e.unwrap().file_name().to_string_lossy().contains("skillark-")));
    }

    #[test]
    fn install_replaces_managed_target_and_keeps_hash() {
        let driver = CopyDriver::new();
        let root = unique_tmp("replace");
        let src = root.join("src");
        std::fs::create_dir_all(&src).unwrap();
        let hash = source_skill(&src, "---\nname: a\nversion: 1\n---\n");
        let target = root.join("tgt");
        std::fs::create_dir_all(&target).unwrap();
        write(&target.join("old.txt"), "stale");

        driver.install(req("op2", &src, &target, &hash)).unwrap();
        // New content present, old gone, hash matches.
        assert!(target.join("SKILL.md").is_file());
        assert!(!target.join("old.txt").exists());
        assert_eq!(hash_directory(&target).unwrap(), hash);
    }

    #[test]
    fn install_rejects_hash_mismatch_without_touching_target() {
        let driver = CopyDriver::new();
        let root = unique_tmp("mismatch");
        let src = root.join("src");
        std::fs::create_dir_all(&src).unwrap();
        source_skill(&src, "---\nname: a\nversion: 1\n---\n");
        let target = root.join("tgt");
        std::fs::create_dir_all(&target).unwrap();
        write(&target.join("preserve.txt"), "keep-me");

        let err = driver
            .install(req("op3", &src, &target, &"0".repeat(64)))
            .expect_err("wrong hash must fail");
        assert!(matches!(err, DeploymentError::HashMismatch { .. }));
        // Original target untouched.
        assert_eq!(
            std::fs::read_to_string(target.join("preserve.txt")).unwrap(),
            "keep-me"
        );
    }

    #[test]
    fn install_clears_stale_tmp_and_backup_from_a_crashed_prior_run() {
        let driver = CopyDriver::new();
        let root = unique_tmp("stale");
        let src = root.join("src");
        std::fs::create_dir_all(&src).unwrap();
        let hash = source_skill(&src, "---\nname: a\nversion: 1\n---\n");
        let target = root.join("tgt");

        // Simulate a crashed previous run leaving debris at the sibling paths.
        let stale_tmp = tmp_path_for(&target, "op5");
        let stale_backup = backup_path_for(&target, "op5");
        std::fs::create_dir_all(&stale_tmp).unwrap();
        write(&stale_tmp.join("junk.txt"), "leftover");
        std::fs::create_dir_all(&stale_backup).unwrap();

        driver.install(req("op5", &src, &target, &hash)).expect("install");
        assert!(target.join("SKILL.md").is_file());
        assert!(!stale_tmp.exists(), "stale tmp must be cleared");
        assert!(!stale_backup.exists(), "stale backup must be cleared");
    }

    // ── verify ───────────────────────────────────────────────────────────

    #[test]
    fn verify_synced_when_target_matches_deployed_and_library() {
        let driver = CopyDriver::new();
        let root = unique_tmp("v-sync");
        let src = root.join("src");
        std::fs::create_dir_all(&src).unwrap();
        let hash = source_skill(&src, "---\nname: a\nversion: 1\n---\n");
        let target = root.join("tgt");
        driver.install(req("op", &src, &target, &hash)).unwrap();

        let rec = make_record(target.clone(), Some(hash.clone()));
        let vr = driver.verify(&rec, &src, &hash).unwrap();
        assert_eq!(vr.status, DeploymentStatus::Synced);
        assert_eq!(vr.reason, DriftReason::None);
    }

    #[test]
    fn verify_outdated_when_library_advanced() {
        let driver = CopyDriver::new();
        let root = unique_tmp("v-out");
        let src = root.join("src");
        std::fs::create_dir_all(&src).unwrap();
        let hash = source_skill(&src, "---\nname: a\nversion: 1\n---\n");
        let target = root.join("tgt");
        driver.install(req("op", &src, &target, &hash)).unwrap();

        // Record says we deployed `hash`, but Library current is a different hash.
        let rec = make_record(target, Some(hash.clone()));
        let other = "1".repeat(64);
        let vr = driver.verify(&rec, &src, &other).unwrap();
        assert_eq!(vr.status, DeploymentStatus::Outdated);
    }

    #[test]
    fn verify_modified_when_user_changed_target() {
        let driver = CopyDriver::new();
        let root = unique_tmp("v-mod");
        let src = root.join("src");
        std::fs::create_dir_all(&src).unwrap();
        let hash = source_skill(&src, "---\nname: a\nversion: 1\n---\n");
        let target = root.join("tgt");
        driver.install(req("op", &src, &target, &hash)).unwrap();

        // User edits the deployed copy.
        write(&target.join("SKILL.md"), "---\nname: a\nversion: 1\n---\nEDITED");

        let rec = make_record(target, Some(hash.clone()));
        let vr = driver.verify(&rec, &src, &hash).unwrap();
        assert_eq!(vr.status, DeploymentStatus::Modified);
        assert_eq!(vr.reason, DriftReason::UserModified);
    }

    #[test]
    fn verify_missing_when_target_gone() {
        let driver = CopyDriver::new();
        let rec = make_record(unique_tmp("v-miss").join("nope"), Some("x".to_owned()));
        let vr = driver.verify(&rec, Path::new("/vault/s"), "x").unwrap();
        assert_eq!(vr.status, DeploymentStatus::Missing);
        assert_eq!(vr.reason, DriftReason::TargetMissing);
    }

    // ── uninstall ────────────────────────────────────────────────────────

    #[test]
    fn uninstall_removes_unmodified_target() {
        let driver = CopyDriver::new();
        let root = unique_tmp("u-clean");
        let src = root.join("src");
        std::fs::create_dir_all(&src).unwrap();
        let hash = source_skill(&src, "---\nname: a\nversion: 1\n---\n");
        let target = root.join("tgt");
        driver.install(req("op", &src, &target, &hash)).unwrap();

        let rec = make_record(target.clone(), Some(hash));
        let res = driver.uninstall(&rec, false).unwrap();
        assert!(res.removed_target);
        assert_eq!(res.status, DeploymentStatus::Uninstalled);
        assert!(!target.exists());
    }

    #[test]
    fn uninstall_refuses_modified_target_without_force() {
        let driver = CopyDriver::new();
        let root = unique_tmp("u-mod");
        let src = root.join("src");
        std::fs::create_dir_all(&src).unwrap();
        let hash = source_skill(&src, "---\nname: a\nversion: 1\n---\n");
        let target = root.join("tgt");
        driver.install(req("op", &src, &target, &hash)).unwrap();
        write(&target.join("SKILL.md"), "---\nname: a\nversion: 1\n---\nEDITED");

        let rec = make_record(target.clone(), Some(hash));
        let res = driver.uninstall(&rec, false).unwrap();
        assert!(!res.removed_target);
        assert_eq!(res.status, DeploymentStatus::Modified);
        assert!(target.exists(), "modified target must be preserved");

        // Force removes it.
        let res2 = driver.uninstall(&rec, true).unwrap();
        assert!(res2.removed_target);
        assert!(!target.exists());
    }

    #[test]
    fn uninstall_idempotent_when_absent() {
        let driver = CopyDriver::new();
        let rec = make_record(unique_tmp("u-absent").join("nope"), Some("x".to_owned()));
        let res = driver.uninstall(&rec, false).unwrap();
        assert!(!res.removed_target);
        assert_eq!(res.status, DeploymentStatus::Uninstalled);
    }
}
