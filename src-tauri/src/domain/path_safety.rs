//! Path safety utilities.
//!
//! These helpers enforce that every path SkillArk reads or writes stays inside
//! a trusted root. They guard the hash/deploy pipelines against directory
//! traversal and symlink-escape attacks:
//!
//! - [`is_within`] — canonical-containment check (resolves symlinks/junctions).
//! - [`has_no_traversal`] — rejects literal `..` components without touching the FS.
//! - [`symlink_escapes_root`] — flags a symlink whose target leaves the root.
//!
//! All functions are pure (no I/O side effects beyond `canonicalize`/`read_link`)
//! and free of Tauri dependencies so they remain unit-testable in the domain layer.

use std::path::{Component, Path};

/// Check if a resolved path is contained within the given root.
///
/// Both sides are canonicalized first, so a symlink or junction that points
/// outside `root` is detected. Returns `false` if either path cannot be
/// canonicalized (e.g. does not exist yet).
pub fn is_within(root: &Path, target: &Path) -> bool {
    let canonical_root = match root.canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };
    let canonical_target = match target.canonicalize() {
        Ok(p) => p,
        Err(_) => return false,
    };
    canonical_target.starts_with(&canonical_root)
}

/// Validate that a path has no `..` components (prevents directory traversal).
///
/// This is a cheap, FS-free check intended for untrusted user input before any
/// filesystem access. It does NOT detect symlink escapes — pair with
/// [`is_within`] / [`symlink_escapes_root`] once the path is materialized.
pub fn has_no_traversal(path: &Path) -> bool {
    !path
        .components()
        .any(|c| matches!(c, Component::ParentDir))
}

/// Reject deployment targets that can never be valid Skill directories.
///
/// This final guard rejects relative/traversing paths, a volume root, the
/// user's home directory itself, and Windows system/application roots.
pub fn validate_deployment_target(target: &Path) -> Result<(), String> {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(std::path::PathBuf::from);
    let protected = [
        std::env::var_os("SystemRoot"),
        std::env::var_os("ProgramFiles"),
        std::env::var_os("ProgramFiles(x86)"),
        std::env::var_os("ProgramData"),
    ]
    .into_iter()
    .flatten()
    .map(std::path::PathBuf::from)
    .collect::<Vec<_>>();
    validate_deployment_target_against(target, home.as_deref(), &protected)
}

fn validate_deployment_target_against(
    target: &Path,
    home: Option<&Path>,
    protected_roots: &[std::path::PathBuf],
) -> Result<(), String> {
    if !target.is_absolute() {
        return Err("deployment target must be an absolute path".to_owned());
    }
    if !has_no_traversal(target) {
        return Err("deployment target must not contain parent traversal".to_owned());
    }
    if target.parent().is_none() {
        return Err("deployment target must not be a volume root".to_owned());
    }

    let normalized = normalized_compare_path(target);
    if home
        .map(normalized_compare_path)
        .is_some_and(|candidate| candidate == normalized)
    {
        return Err("deployment target must not be the user home directory".to_owned());
    }
    if protected_roots.iter().any(|root| {
        let root = normalized_compare_path(root);
        normalized == root || normalized.starts_with(&(root + "/"))
    }) {
        return Err("deployment target must not be inside a protected system directory".to_owned());
    }
    Ok(())
}

fn normalized_compare_path(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

/// Check if a path is a symlink whose target escapes the root.
///
/// Returns `false` when `link_path` is not a symlink, or when the symlink
/// resolves to a location inside `root`. Returns `true` (fail-closed) when the
/// link target resolves outside `root`, or when the symlink itself cannot be
/// read (treat unreadable as dangerous).
pub fn symlink_escapes_root(root: &Path, link_path: &Path) -> bool {
    if !link_path.is_symlink() {
        return false;
    }
    match std::fs::read_link(link_path) {
        Ok(target) => {
            let resolved = if target.is_absolute() {
                target
            } else {
                // Resolve relative to the link's parent directory.
                link_path
                    .parent()
                    .unwrap_or_else(|| Path::new(""))
                    .join(target)
            };
            !is_within(root, &resolved)
        }
        Err(_) => true, // Can't read symlink -> assume dangerous.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn unique_tmp(sub: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "skillark-path-{}-{}-{sub}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn traversal_detected() {
        // A literal `..` must be flagged without touching the filesystem.
        assert!(!has_no_traversal(Path::new("../escape")));
        assert!(!has_no_traversal(Path::new("a/../../etc")));
        assert!(!has_no_traversal(Path::new("a/../b/..")));
    }

    #[test]
    fn normal_path_passes() {
        // Ordinary descendant paths contain no `..`.
        assert!(has_no_traversal(Path::new("skills/sample/SKILL.md")));
        assert!(has_no_traversal(Path::new("a/b/c")));
        assert!(has_no_traversal(Path::new("SKILL.md")));
    }

    #[test]
    fn deployment_target_rejects_relative_root_home_and_system_paths() {
        let home = Path::new("C:/Users/example");
        let protected = vec![PathBuf::from("C:/Windows"), PathBuf::from("C:/Program Files")];

        assert!(validate_deployment_target_against(
            Path::new("relative/skill"),
            Some(home),
            &protected
        )
        .is_err());
        assert!(
            validate_deployment_target_against(Path::new("C:/"), Some(home), &protected).is_err()
        );
        assert!(validate_deployment_target_against(home, Some(home), &protected).is_err());
        assert!(validate_deployment_target_against(
            Path::new("C:/Windows/System32/skill"),
            Some(home),
            &protected
        )
        .is_err());
        assert!(validate_deployment_target_against(
            Path::new("C:/Users/example/.codex/skills/demo"),
            Some(home),
            &protected
        )
        .is_ok());
    }

    #[test]
    fn is_within_allows_descendant() {
        let root = unique_tmp("within-ok");
        let child = root.join("nested/file.md");
        fs::create_dir_all(child.parent().unwrap()).unwrap();
        fs::write(&child, "x").unwrap();

        assert!(is_within(&root, &child), "child must be within root");
    }

    #[test]
    fn is_within_rejects_outside() {
        let root_a = unique_tmp("within-a");
        let root_b = unique_tmp("within-b");
        let file_in_b = root_b.join("secret.md");
        fs::write(&file_in_b, "x").unwrap();

        assert!(
            !is_within(&root_a, &file_in_b),
            "sibling root must not be considered within"
        );
    }

    #[test]
    fn non_symlink_not_flagged() {
        let root = unique_tmp("nonsym");
        let regular = root.join("plain.txt");
        fs::write(&regular, "x").unwrap();

        assert!(
            !symlink_escapes_root(&root, &regular),
            "regular file must never be flagged as an escaping symlink"
        );
    }

    // Symlink creation needs elevated privileges on Windows; only assert on unix.
    #[cfg(unix)]
    #[test]
    fn symlink_escape_detected() {
        use std::os::unix::fs::symlink;

        let root = unique_tmp("symescape-root");
        let outside = unique_tmp("symescape-outside");
        let outside_file = outside.join("outside.txt");
        fs::write(&outside_file, "external").unwrap();

        // A symlink inside `root` that points to a file outside `root`.
        let escaping_link = root.join("escape-link");
        symlink(&outside_file, &escaping_link).unwrap();

        assert!(
            symlink_escapes_root(&root, &escaping_link),
            "symlink pointing outside root must be detected as an escape"
        );

        // A symlink that stays inside the root must NOT be flagged.
        let inside_target = root.join("inside-target.txt");
        fs::write(&inside_target, "internal").unwrap();
        let internal_link = root.join("internal-link");
        symlink(&inside_target, &internal_link).unwrap();
        assert!(
            !symlink_escapes_root(&root, &internal_link),
            "symlink staying inside root must not be flagged"
        );
    }
}
