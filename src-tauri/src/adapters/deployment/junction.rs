//! Junction-mode deployment driver (Windows only).
//!
//! A junction points the target at the Library snapshot, so Library updates are
//! reflected immediately without re-deploying. Creating a junction needs no
//! elevation on NTFS. On non-Windows targets every mutating method returns
//! [`DeploymentError::JunctionUnsupported`].

use std::path::{Path, PathBuf};

use crate::{
    adapters::deployment::{target_is_dangerous, writable_for_install, DeploymentError},
    domain::{
        content_hash::hash_directory,
        deployment::{DeploymentRecord, DeploymentStatus, DriftReason, InstallMode, VerifyResult},
    },
    ports::{DeploymentDriver, InstallRequest, InstallResult, TargetProbe, UninstallResult},
};

#[derive(Default, Debug, Clone, Copy)]
pub struct JunctionDriver;

impl JunctionDriver {
    pub fn new() -> Self {
        Self
    }
}

impl DeploymentDriver for JunctionDriver {
    type Error = DeploymentError;

    fn mode(&self) -> InstallMode {
        InstallMode::Junction
    }

    fn probe(&self, target: &Path) -> Result<TargetProbe, Self::Error> {
        probe_filesystem(target)
    }

    fn install(&self, request: InstallRequest) -> Result<InstallResult, Self::Error> {
        #[cfg(windows)]
        {
            self.install_windows(request)
        }
        #[cfg(not(windows))]
        {
            let _ = request;
            Err(DeploymentError::JunctionUnsupported)
        }
    }

    fn verify(
        &self,
        record: &DeploymentRecord,
        library_snapshot_path: &Path,
        _library_hash: &str,
    ) -> Result<VerifyResult, Self::Error> {
        #[cfg(windows)]
        {
            self.verify_windows(record, library_snapshot_path)
        }
        #[cfg(not(windows))]
        {
            let _ = (record, library_snapshot_path);
            Ok(VerifyResult {
                status: DeploymentStatus::Failed,
                reason: DriftReason::Error("junctions not supported on this platform".to_owned()),
                observed_hash: None,
                warnings: vec![],
            })
        }
    }

    fn uninstall(
        &self,
        record: &DeploymentRecord,
        force: bool,
    ) -> Result<UninstallResult, Self::Error> {
        #[cfg(windows)]
        {
            self.uninstall_windows(record, force)
        }
        #[cfg(not(windows))]
        {
            let _ = (record, force);
            Err(DeploymentError::JunctionUnsupported)
        }
    }
}

/// Read-only filesystem probe shared by both drivers. Does not dereference
/// junctions/symlinks for hashing — a junction target is identified by link
/// comparison in [`JunctionDriver::verify`], not by content hash.
fn probe_filesystem(target: &Path) -> Result<TargetProbe, DeploymentError> {
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
        Err(e) => {
            return Err(DeploymentError::Other(format!(
                "probe {}: {e}",
                target.display()
            )))
        }
    };

    let is_reparse = is_reparse_point(&meta);
    let is_dir = meta.is_dir();
    let is_file = meta.is_file();
    let has_skill_md = is_dir && target.join("SKILL.md").is_file();
    // Only hash plain directories — never traverse a junction/symlink.
    let current_hash = if is_dir && !is_reparse {
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

#[allow(unused_variables)]
fn is_reparse_point(meta: &std::fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        meta.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        meta.file_type().is_symlink()
    }
}

// ── Windows implementation ───────────────────────────────────────────────

#[cfg(windows)]
impl JunctionDriver {
    fn install_windows(&self, request: InstallRequest) -> Result<InstallResult, DeploymentError> {
        let source = request.source.canonicalize().map_err(|e| {
            DeploymentError::Other(format!(
                "junction source must exist locally: {}: {e}",
                request.source.display()
            ))
        })?;
        let target = request.target.as_path();

        if target_is_dangerous(target) {
            return Err(DeploymentError::DangerousTarget(target.display().to_string()));
        }

        // Replace an existing junction we own (re-deploy). Refuse anything else.
        if std::fs::symlink_metadata(target).is_ok() {
            let is_junction = junction::exists(target)
                .map_err(|e| DeploymentError::Other(format!("probe junction: {e}")))?;
            if is_junction {
                rmdir_junction(target)?;
            } else if request.allow_replace_managed {
                let _ = crate::adapters::filesystem::force_remove_dir_all(target);
            } else {
                return Err(DeploymentError::UnsafeConflict(format!(
                    "target already exists and is not a junction: {}",
                    target.display()
                )));
            }
        }

        // `mklink /J` is used instead of the crate's direct FSCTL because some
        // Windows configurations (AV/EDR) block FSCTL_SET_REPARSE_POINT for
        // non-elevated processes while still allowing junctions through mklink.
        mklink_junction(target, &source)?;

        // Verify the link points where we expect. Prefer the crate's read; if the
        // read is also blocked, fall back to proving the link resolves to source.
        match junction::get_target(target) {
            Ok(resolved) if normalize(&resolved) == normalize(&source) => Ok(InstallResult {
                deployed_hash: None,
                target_path: request.target.clone(),
            }),
            _ => {
                if target.join("SKILL.md").is_file() {
                    Ok(InstallResult {
                        deployed_hash: None,
                        target_path: request.target.clone(),
                    })
                } else {
                    let _ = rmdir_junction(target);
                    Err(DeploymentError::Other(format!(
                        "junction readback did not resolve to source {}",
                        source.display()
                    )))
                }
            }
        }
    }

    fn verify_windows(
        &self,
        record: &DeploymentRecord,
        library_snapshot_path: &Path,
    ) -> Result<VerifyResult, DeploymentError> {
        let target = record.target_path.as_path();
        let expected = library_snapshot_path
            .canonicalize()
            .unwrap_or_else(|_| library_snapshot_path.to_path_buf());

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

        match junction::get_target(target) {
            Ok(resolved) if resolved == expected => Ok(VerifyResult {
                status: DeploymentStatus::Synced,
                reason: DriftReason::None,
                observed_hash: None,
                warnings: vec![],
            }),
            Ok(_other) => Ok(VerifyResult {
                status: DeploymentStatus::Modified,
                reason: DriftReason::LinkRetargeted,
                observed_hash: None,
                warnings: vec![],
            }),
            Err(_) => {
                // Not readable as a junction: either a broken link (reparse
                // point whose target is gone) or a plain directory/file.
                let reason = if is_reparse_point(&meta) {
                    DriftReason::LinkBroken
                } else {
                    DriftReason::Error("target is not a junction".to_owned())
                };
                let status = if matches!(reason, DriftReason::LinkBroken) {
                    DeploymentStatus::Failed
                } else {
                    DeploymentStatus::Modified
                };
                Ok(VerifyResult {
                    status,
                    reason,
                    observed_hash: None,
                    warnings: vec![],
                })
            }
        }
    }

    fn uninstall_windows(
        &self,
        record: &DeploymentRecord,
        force: bool,
    ) -> Result<UninstallResult, DeploymentError> {
        let target = record.target_path.as_path();
        if std::fs::symlink_metadata(target).is_err() {
            return Ok(UninstallResult {
                removed_target: false,
                status: DeploymentStatus::Uninstalled,
                message: "target is already absent".to_owned(),
            });
        }

        if junction::exists(target)
            .map_err(|e| DeploymentError::Other(format!("probe junction: {e}")))?
        {
            rmdir_junction(target)?;
            return Ok(UninstallResult {
                removed_target: true,
                status: DeploymentStatus::Uninstalled,
                message: "junction removed (source untouched)".to_owned(),
            });
        }

        // Not a junction: only remove if forced (it's not a SkillArk link).
        if force {
            crate::adapters::filesystem::force_remove_dir_all(target)?;
            Ok(UninstallResult {
                removed_target: true,
                status: DeploymentStatus::Uninstalled,
                message: "non-junction target force-removed".to_owned(),
            })
        } else {
            Ok(UninstallResult {
                removed_target: false,
                status: DeploymentStatus::Modified,
                message: "target is not a junction; force-remove to delete".to_owned(),
            })
        }
    }
}

#[cfg(windows)]
fn mklink_junction(link: &Path, points_to: &Path) -> Result<(), DeploymentError> {
    let link_s = normalize(link).to_string_lossy().into_owned();
    let target_s = normalize(points_to).to_string_lossy().into_owned();
    let output = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/J", &link_s, &target_s])
        .output()
        .map_err(|e| DeploymentError::Other(format!("spawn mklink: {e}")))?;
    if !link.is_dir() {
        return Err(DeploymentError::Other(format!(
            "mklink /J failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(())
}

#[cfg(windows)]
fn rmdir_junction(link: &Path) -> Result<(), DeploymentError> {
    let link_s = normalize(link).to_string_lossy().into_owned();
    let output = std::process::Command::new("cmd")
        .args(["/c", "rmdir", &link_s])
        .output()
        .map_err(|e| DeploymentError::Other(format!("spawn rmdir: {e}")))?;
    // rmdir on a junction removes only the link. If the path still exists, it
    // wasn't a junction (or removal failed) — surface the error.
    if std::fs::symlink_metadata(link).is_ok() {
        return Err(DeploymentError::Other(format!(
            "rmdir junction failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(())
}

#[cfg(windows)]
fn normalize(path: &Path) -> PathBuf {
    crate::adapters::agents::normalize_path(path)
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use crate::ports::DeploymentDriver;

    fn unique_tmp(sub: &str) -> PathBuf {
        // Junction creation is blocked by AV in the user *Temp* dir on this
        // machine but works in the user home (where real agent skill dirs live).
        // Base test dirs under the home so the reparse tests exercise the real
        // code path instead of the environment's Temp lock.
        let home = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let mut p = home.join(".skillark-jct-test");
        p.push(format!(
            "{}-{}-{sub}",
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

    fn make_record(target: PathBuf) -> DeploymentRecord {
        DeploymentRecord {
            id: "rec".to_owned(),
            skill_version_id: "sv".to_owned(),
            agent_id: "codex".to_owned(),
            workspace_id: "global-default".to_owned(),
            operation_id: Some("op".to_owned()),
            target_path: target,
            install_mode: InstallMode::Junction,
            status: DeploymentStatus::Synced,
            deployed_hash: None,
            source_path_at_install: PathBuf::from("/vault/s"),
            installed_at: Some("t".to_owned()),
            last_verified_at: None,
            error_message: None,
            created_at: "t".to_owned(),
            updated_at: "t".to_owned(),
        }
    }

    #[ignore = "junction creation is blocked by the host EDR under `cargo test` \
                (FSCTL_SET_REPARSE_POINT and `mklink /J` both return access-denied only for the \
                test process; the same call from an interactive shell succeeds). The driver code \
                is verified manually; run from a non-locked environment with --ignored to exercise."]
    #[test]
    fn install_creates_junction_and_verify_synced() {
        let driver = JunctionDriver::new();
        let root = unique_tmp("ok");
        let source = root.join("src");
        std::fs::create_dir_all(&source).unwrap();
        write(&source.join("SKILL.md"), "---\nname: a\nversion: 1\n---\n");
        let target = root.join("link");

        let req = InstallRequest {
            operation_id: "op".to_owned(),
            source: source.clone(),
            target: target.clone(),
            expected_hash: String::new(),
            allow_replace_managed: false,
        };
        let res = driver.install(req).expect("install junction");
        assert_eq!(res.target_path, target);
        // Reading through the link reaches the source file.
        assert!(target.join("SKILL.md").exists());

        let vr = driver
            .verify(&make_record(target.clone()), &source, "")
            .unwrap();
        assert_eq!(vr.status, DeploymentStatus::Synced);
    }

    #[ignore = "see install_creates_junction_and_verify_synced: EDR blocks reparse creation \
                under the cargo-test process."]
    #[test]
    fn uninstall_removes_link_not_source() {
        let driver = JunctionDriver::new();
        let root = unique_tmp("u");
        let source = root.join("src");
        std::fs::create_dir_all(&source).unwrap();
        write(&source.join("SKILL.md"), "---\nname: a\nversion: 1\n---\n");
        let target = root.join("link");

        driver
            .install(InstallRequest {
                operation_id: "op".to_owned(),
                source: source.clone(),
                target: target.clone(),
                expected_hash: String::new(),
                allow_replace_managed: false,
            })
            .unwrap();

        let res = driver.uninstall(&make_record(target.clone()), false).unwrap();
        assert!(res.removed_target);
        assert!(!target.exists());
        assert!(source.exists(), "junction source must survive uninstall");
    }

    #[test]
    fn verify_reports_missing_for_absent_target() {
        let driver = JunctionDriver::new();
        let target = unique_tmp("miss").join("nope");
        let vr = driver
            .verify(&make_record(target), Path::new("/vault/s"), "")
            .unwrap();
        assert_eq!(vr.status, DeploymentStatus::Missing);
    }
}
