//! Skill source adapters — turn an on-disk origin into a parsed skill.
//!
//! Two origins ship in v0.1: a local directory ([`local_dir::LocalDirectorySource`])
//! and a ZIP archive ([`zip::ZipSource`]). ZIP is first extracted into a scratch
//! directory with Zip-Slip protection, then the located skill root is scanned by
//! the local-directory adapter — so the manifest parsing path is shared.

use std::fs;
use std::path::{Path, PathBuf};

pub mod local_dir;
pub mod zip;
pub mod git;

pub use local_dir::LocalDirectorySource;
pub use zip::ZipSource;

#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error("SKILL.md not found in {0}")]
    NoSkillMd(PathBuf),
    #[error(transparent)]
    Parse(#[from] crate::domain::skill_manifest::ParseError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("zip entry `{0}` escapes the extract root (Zip Slip rejected)")]
    ZipSlip(String),
    #[error("zip read error: {0}")]
    Zip(String),
}

/// Locate the directory that holds `SKILL.md`.
///
/// Handles two common layouts:
/// 1. `dir/SKILL.md` — the picked directory *is* the skill root.
/// 2. `dir/<single-folder>/SKILL.md` — archives often wrap one top folder.
///
/// Anything else (no SKILL.md, or ambiguous multi-folder layouts) is rejected.
pub fn locate_skill_root(dir: &Path) -> Result<PathBuf, SourceError> {
    if dir.join("SKILL.md").is_file() {
        return Ok(dir.to_path_buf());
    }

    let mut entries: Vec<_> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    if entries.len() == 1 && entries[0].is_dir() && entries[0].join("SKILL.md").is_file() {
        return Ok(entries.pop().unwrap());
    }

    Err(SourceError::NoSkillMd(dir.to_path_buf()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_tmp(sub: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "skillark-src-{}-{}-{sub}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn locate_root_when_skill_md_at_top() {
        let dir = unique_tmp("top");
        std::fs::write(dir.join("SKILL.md"), "---\nname: a\nversion: 1\n---\n").unwrap();
        assert_eq!(locate_skill_root(&dir).unwrap(), dir);
    }

    #[test]
    fn locate_root_when_wrapped_in_single_folder() {
        let dir = unique_tmp("wrapped");
        let inner = dir.join("the-skill");
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::write(inner.join("SKILL.md"), "---\nname: a\nversion: 1\n---\n").unwrap();
        assert_eq!(locate_skill_root(&dir).unwrap(), inner);
    }

    #[test]
    fn locate_root_rejects_missing_skill_md() {
        let dir = unique_tmp("missing");
        std::fs::write(dir.join("README.md"), "nope").unwrap();
        assert!(matches!(locate_skill_root(&dir), Err(SourceError::NoSkillMd(_))));
    }

    #[test]
    fn locate_root_rejects_ambiguous_multi_folder() {
        let dir = unique_tmp("ambiguous");
        for name in ["a", "b"] {
            let inner = dir.join(name);
            std::fs::create_dir_all(&inner).unwrap();
            std::fs::write(inner.join("SKILL.md"), "---\nname: x\nversion: 1\n---\n").unwrap();
        }
        assert!(matches!(locate_skill_root(&dir), Err(SourceError::NoSkillMd(_))));
    }
}
