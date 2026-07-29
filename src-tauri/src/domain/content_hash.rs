//! Deterministic directory hashing.
//!
//! [`hash_directory`] produces a stable SHA-256 digest of a directory tree.
//! The hash is **content-addressed**: only regular file bytes and their
//! repository-relative paths matter. Metadata such as mtime, permissions, or
//! directory structure churn that does not change file contents must not change
//! the hash. This is the foundation for SkillArk's "fixed fixture hashes
//! identically across repeated runs" acceptance criterion.
//!
//! Security: symlinks and Windows reparse points are skipped entirely so a
//! malicious skill directory cannot smuggle in (or escape to) files outside the
//! hashed root. See [`crate::domain::path_safety`] for the traversal guards.

use std::{collections::BTreeMap, fs, path::Path};

use sha2::{Digest, Sha256};

/// Compute a deterministic SHA-256 hash of a directory.
///
/// Invariants:
/// - Only regular files are included (no symlinks / junctions / reparse points).
/// - Symlinks and reparse points that escape the root are skipped.
/// - File contents are hashed, sorted by relative path (forward-slash form).
/// - `mtime` / permissions are NOT included.
///
/// Returns the digest as a lowercase hex string.
pub fn hash_directory(root: &Path) -> Result<String, String> {
    let mut entries: BTreeMap<String, Vec<u8>> = BTreeMap::new();
    collect_files(root, root, &mut entries)?;

    let mut hasher = Sha256::new();
    for (relative_path, content) in &entries {
        hasher.update(relative_path.as_bytes());
        hasher.update(b"\0");
        hasher.update(content);
        hasher.update(b"\0");
    }
    Ok(format!("{:x}", hasher.finalize()))
}

/// Recursively walk `current`, recording every regular file under `root` into
/// `entries` keyed by its forward-slash relative path.
///
/// `strip_prefix(root, ...)` guarantees that any path that somehow resolves
/// outside the root fails the whole hash rather than silently leaking an
/// absolute or escaping relative path into the digest.
fn collect_files(
    root: &Path,
    current: &Path,
    entries: &mut BTreeMap<String, Vec<u8>>,
) -> Result<(), String> {
    // Read dir entries, sort for determinism independent of FS order.
    let mut items: Vec<_> = fs::read_dir(current)
        .map_err(|e| format!("failed to read {}: {e}", current.display()))?
        .filter_map(|e| e.ok())
        .collect();
    items.sort_by_key(|e| e.file_name());

    for entry in items {
        let path = entry.path();
        let meta = match fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(_) => continue, // unreadable entry: skip, don't abort.
        };

        // Skip ALL symlinks for safety (see module docs).
        if meta.file_type().is_symlink() {
            continue;
        }

        #[cfg(windows)]
        {
            // Skip reparse points (junctions, mounted volume pseudo-links, ...).
            use std::os::windows::fs::MetadataExt;
            // FILE_ATTRIBUTE_REPARSE_POINT == 0x400
            if meta.file_attributes() & 0x400 != 0 {
                continue;
            }
        }

        if meta.is_dir() {
            collect_files(root, &path, entries)?;
        } else if meta.is_file() {
            let relative = path
                .strip_prefix(root)
                .map_err(|e| format!("path escapes root {}: {e}", root.display()))?;
            // Normalize separators so a fixture hashed on Windows and Linux
            // produces the same digest.
            let relative_str = relative.to_string_lossy().replace('\\', "/");
            let content = fs::read(&path)
                .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
            entries.insert(relative_str, content);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    /// Unique temp dir per test process, so parallel `cargo test` never clashes.
    fn unique_tmp(sub: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "skillark-hash-{}-{}-{sub}",
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
    fn hash_is_deterministic() {
        let dir = unique_tmp("deterministic");
        write(&dir.join("a.txt"), "alpha");
        write(&dir.join("nested/b.txt"), "beta");

        let h1 = hash_directory(&dir).expect("hash once");
        let h2 = hash_directory(&dir).expect("hash twice");

        assert_eq!(h1, h2, "same contents must hash identically");
        assert_eq!(h1.len(), 64, "sha-256 hex is 64 chars");
    }

    #[test]
    fn hash_changes_when_content_changes() {
        let dir = unique_tmp("content-change");
        write(&dir.join("a.txt"), "alpha");

        let before = hash_directory(&dir).unwrap();
        write(&dir.join("a.txt"), "ALPHA");
        let after = hash_directory(&dir).unwrap();

        assert_ne!(before, after, "changing bytes must change the hash");
    }

    #[test]
    fn hash_ignores_mtime() {
        let dir = unique_tmp("mtime");
        write(&dir.join("a.txt"), "stable content");

        let before = hash_directory(&dir).unwrap();

        // Mutate only the mtime, keep bytes identical.
        let file = dir.join("a.txt");
        let new_time = std::time::SystemTime::now() - std::time::Duration::from_secs(86_400);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // Also toggle permissions to prove metadata is ignored.
            let mut perms = fs::metadata(&file).unwrap().permissions();
            perms.set_mode(0o600);
            let _ = fs::set_permissions(&file, perms);
        }
        {
            use std::fs::OpenOptions;
            let f = OpenOptions::new().write(true).open(&file).unwrap();
            // FileTimes is stable since Rust 1.75.
            let times = std::fs::FileTimes::new().set_modified(new_time);
            let _ = f.set_times(times);
        }

        let after = hash_directory(&dir).unwrap();
        assert_eq!(before, after, "mtime/permission changes must not affect hash");
    }

    #[test]
    fn hash_sorts_by_path() {
        // Add files in reverse lexicographic order; BTreeMap must canonicalize.
        let dir_a = unique_tmp("sort-a");
        let dir_b = unique_tmp("sort-b");

        write(&dir_a.join("z.txt"), "zeta");
        write(&dir_a.join("a.txt"), "alpha");
        write(&dir_a.join("m.txt"), "mu");

        write(&dir_b.join("a.txt"), "alpha");
        write(&dir_b.join("m.txt"), "mu");
        write(&dir_b.join("z.txt"), "zeta");

        assert_eq!(
            hash_directory(&dir_a).unwrap(),
            hash_directory(&dir_b).unwrap(),
            "insertion order must not matter"
        );
    }

    // Symlinks need elevated privileges on Windows, so only assert on unix.
    #[cfg(unix)]
    #[test]
    fn hash_ignores_symlinks() {
        use std::os::unix::fs::symlink;

        let dir = unique_tmp("symlink");
        write(&dir.join("real.txt"), "payload");

        // A symlink inside the hashed dir must not perturb the digest.
        symlink("real.txt", dir.join("link.txt")).unwrap();

        let with_link = hash_directory(&dir).unwrap();

        let clean = unique_tmp("symlink-clean");
        write(&clean.join("real.txt"), "payload");
        let without_link = hash_directory(&clean).unwrap();

        assert_eq!(
            with_link, without_link,
            "symlinks must be excluded from the hash"
        );
    }
}
