//! Filesystem primitives shared by the import and deployment adapters.
//!
//! The central invariant these helpers uphold: **copying a skill tree must not
//! change its content hash.** [`domain::content_hash::hash_directory`] skips
//! symlinks and Windows reparse points, so [`copy_tree`] skips them too —
//! otherwise the installed copy would hash differently from the source and
//! verification could never report `synced`.
//!
//! All paths are treated as opaque bytes; callers are responsible for having
//! canonicalized/validated them via [`domain::path_safety`] first.

use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, thiserror::Error)]
pub enum FsError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

impl FsError {
    pub fn with_context(self, ctx: impl Into<String>) -> Self {
        match self {
            Self::Io(source) => Self::Other(format!("{}: {source}", ctx.into())),
            other => other,
        }
    }
}

pub type FsResult<T> = Result<T, FsError>;

/// Recursively copy the regular-file tree from `source` into `target`.
///
/// - Directories are recreated.
/// - Symlinks and Windows reparse points (junctions, mounted-volume links) are
///   skipped, matching `hash_directory`, so the copy hashes identically.
/// - `target` is created if missing; it must not already exist as a file.
pub fn copy_tree(source: &Path, target: &Path) -> FsResult<()> {
    if !source.is_dir() {
        return Err(FsError::Other(format!(
            "source is not a directory: {}",
            source.display()
        )));
    }
    fs::create_dir_all(target)
        .map_err(|e| FsError::Io(e).with_context(format!("create {}", target.display())))?;
    copy_inner(source, target)?;
    Ok(())
}

fn copy_inner(source: &Path, target: &Path) -> FsResult<()> {
    let mut entries: Vec<_> = fs::read_dir(source)
        .map_err(|e| FsError::Io(e).with_context(format!("read {}", source.display())))?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        let meta = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(_) => continue, // unreadable entry: skip, don't abort the whole copy.
        };

        // Skip ALL symlinks (see module docs: hash parity + escape safety).
        if meta.file_type().is_symlink() {
            continue;
        }

        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            // FILE_ATTRIBUTE_REPARSE_POINT == 0x400
            if meta.file_attributes() & 0x400 != 0 {
                continue;
            }
        }

        let name = entry.file_name();
        let dest = target.join(&name);
        if meta.is_dir() {
            fs::create_dir_all(&dest).map_err(FsError::from)?;
            copy_inner(&path, &dest)?;
        } else if meta.is_file() {
            fs::copy(&path, &dest).map_err(FsError::from)?;
        }
    }
    Ok(())
}

/// Same-volume rename. The standard-library rename is atomic within a volume on
/// both Windows and Unix; renaming across volumes fails. Callers must keep the
/// temporary/backup paths as siblings of the target so they share a volume.
pub fn rename_atomic(from: &Path, to: &Path) -> FsResult<()> {
    fs::rename(from, to).map_err(|e| {
        FsError::Other(format!(
            "rename {} -> {} failed (often a cross-volume move): {e}",
            from.display(),
            to.display()
        ))
    })
}

/// Remove a directory tree, clearing the read-only attribute on Windows where
/// `remove_dir_all` would otherwise refuse. No-op if the path does not exist.
pub fn force_remove_dir_all(path: &Path) -> FsResult<()> {
    if fs::symlink_metadata(path).is_err() {
        return Ok(());
    }
    clear_readonly_recursive(path);
    fs::remove_dir_all(path)
        .map_err(|e| FsError::Io(e).with_context(format!("remove {}", path.display())))?;
    Ok(())
}

fn clear_readonly_recursive(path: &Path) {
    // Symlink_metadata so we never follow a trailing symlink and chmod its target.
    let meta = match fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(_) => return,
    };
    if meta.is_dir() {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                clear_readonly_recursive(&entry.path());
            }
        }
    }
    let _ = make_writable(path);
}

// `set_readonly(false)` is exactly the right call to clear the read-only
// attribute that blocks `remove_dir_all` on Windows; clippy's suggestion to use
// PermissionsExt does not apply to our cross-platform intent.
#[allow(clippy::permissions_set_readonly_false)]
fn make_writable(path: &Path) -> FsResult<()> {
    let mut perms = fs::metadata(path)
        .map_err(|e| FsError::Io(e).with_context(format!("stat {}", path.display())))?
        .permissions();
    perms.set_readonly(false);
    fs::set_permissions(path, perms)
        .map_err(|e| FsError::Io(e).with_context(format!("chmod {}", path.display())))?;
    Ok(())
}

/// Sibling temp directory name for an in-flight operation.
pub fn tmp_path_for(target: &Path, operation_id: &str) -> PathBuf {
    let parent = target.parent().unwrap_or_else(|| Path::new(""));
    parent.join(format!(
        "{}.skillark-tmp-{operation_id}",
        safe_stem(target)
    ))
}

/// Sibling backup directory name used while atomically replacing a target.
pub fn backup_path_for(target: &Path, operation_id: &str) -> PathBuf {
    let parent = target.parent().unwrap_or_else(|| Path::new(""));
    parent.join(format!(
        "{}.skillark-backup-{operation_id}",
        safe_stem(target)
    ))
}

fn safe_stem(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "target".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::content_hash::hash_directory;

    fn unique_tmp(sub: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "skillark-fs-{}-{}-{sub}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn write(path: &Path, body: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, body).unwrap();
    }

    #[test]
    fn copy_tree_preserves_hash() {
        let src = unique_tmp("src");
        write(&src.join("SKILL.md"), "---\nname: a\nversion: 1\n---\nbody");
        write(&src.join("scripts/run.sh"), "echo hi");
        write(&src.join("refs/nested/deep.md"), "deep");

        let dst = unique_tmp("dst");
        copy_tree(&src, &dst).expect("copy");

        assert_eq!(
            hash_directory(&src).unwrap(),
            hash_directory(&dst).unwrap(),
            "copied tree must hash identically to its source"
        );
    }

    #[test]
    fn copy_tree_rejects_non_directory_source() {
        let dir = unique_tmp("nondir");
        let file = dir.join("plain.txt");
        write(&file, "x");
        let dst = unique_tmp("nondir-dst");
        assert!(copy_tree(&file, &dst).is_err());
    }

    #[test]
    fn force_remove_clears_readonly_tree() {
        let dir = unique_tmp("ro");
        let file = dir.join("locked.txt");
        write(&file, "x");
        // Mark read-only to prove force_remove_dir_all clears it.
        let mut perms = fs::metadata(&file).unwrap().permissions();
        perms.set_readonly(true);
        fs::set_permissions(&file, perms).unwrap();

        force_remove_dir_all(&dir).expect("removes despite read-only");
        assert!(!dir.exists());
    }

    #[test]
    fn force_remove_is_noop_when_missing() {
        let ghost = unique_tmp("ghost").join("never");
        force_remove_dir_all(&ghost).expect("missing path is a no-op");
    }

    #[test]
    fn rename_atomic_moves_directory() {
        let root = unique_tmp("rename");
        let a = root.join("a");
        let b = root.join("b");
        fs::create_dir_all(&a).unwrap();
        write(&a.join("f.txt"), "payload");

        rename_atomic(&a, &b).expect("same-volume rename");
        assert!(b.is_dir());
        assert!(!a.exists());
        assert_eq!(fs::read_to_string(b.join("f.txt")).unwrap(), "payload");
    }

    #[test]
    fn naming_helpers_are_siblings() {
        let target = PathBuf::from("/vault/x/skill");
        let op = "op-1";
        let tmp = tmp_path_for(&target, op);
        let bak = backup_path_for(&target, op);
        assert_eq!(tmp.parent().unwrap(), target.parent().unwrap());
        assert_eq!(bak.parent().unwrap(), target.parent().unwrap());
        assert!(tmp.to_string_lossy().ends_with("skill.skillark-tmp-op-1"));
        assert!(bak
            .to_string_lossy()
            .ends_with("skill.skillark-backup-op-1"));
    }

    // Symlinks need privileges on Windows; assert the hash-parity contract on unix.
    #[cfg(unix)]
    #[test]
    fn copy_tree_skips_symlinks_like_hash() {
        use std::os::unix::fs::symlink;

        let src = unique_tmp("sym");
        write(&src.join("real.txt"), "payload");
        symlink("real.txt", src.join("link.txt")).unwrap();

        let dst = unique_tmp("sym-dst");
        copy_tree(&src, &dst).unwrap();

        // Hash of the copy (no symlink) must equal hash of the source (symlink skipped).
        assert_eq!(hash_directory(&src).unwrap(), hash_directory(&dst).unwrap());
        // And the link must not have been materialized as a regular file in the copy.
        assert!(!dst.join("link.txt").exists());
    }
}
