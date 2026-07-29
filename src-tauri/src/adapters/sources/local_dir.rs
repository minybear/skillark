//! Local-directory skill source.
//!
//! Scans a directory that contains (or wraps) a `SKILL.md`, returning the
//! parsed manifest and the source root the import service should copy from.

use std::path::PathBuf;

use crate::{
    domain::skill_manifest::parse_skill_md,
    ports::{ScannedSource, SkillSourceAdapter},
};

use super::{locate_skill_root, SourceError};

pub struct LocalDirectorySource {
    root: PathBuf,
}

impl LocalDirectorySource {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl SkillSourceAdapter for LocalDirectorySource {
    type Error = SourceError;

    fn scan(&self) -> Result<ScannedSource, Self::Error> {
        let source_root = locate_skill_root(&self.root)?;
        let skill_md = std::fs::read_to_string(source_root.join("SKILL.md"))?;
        let manifest = parse_skill_md(&skill_md)?;
        Ok(ScannedSource {
            manifest,
            source_root,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_tmp(sub: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "skillark-local-{}-{}-{sub}",
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
    fn scans_skill_md_at_root() {
        let dir = unique_tmp("ok");
        std::fs::write(
            dir.join("SKILL.md"),
            "---\nname: my-skill\nversion: 1.0.0\ndescription: hi\n---\nbody",
        )
        .unwrap();
        std::fs::create_dir_all(dir.join("scripts")).unwrap();
        std::fs::write(dir.join("scripts/run.sh"), "echo").unwrap();

        let scanned = LocalDirectorySource::new(dir.clone()).scan().expect("parses");
        assert_eq!(scanned.manifest.name, "my-skill");
        assert_eq!(scanned.manifest.version, "1.0.0");
        assert_eq!(scanned.source_root, dir);
    }

    #[test]
    fn scans_wrapped_folder() {
        let dir = unique_tmp("wrapped");
        let inner = dir.join("pkg");
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::write(inner.join("SKILL.md"), "---\nname: pkg\nversion: 0.1.0\n---\n").unwrap();

        let scanned = LocalDirectorySource::new(dir).scan().expect("parses wrapped");
        assert_eq!(scanned.source_root, inner);
        assert_eq!(scanned.manifest.name, "pkg");
    }

    #[test]
    fn rejects_missing_skill_md() {
        let dir = unique_tmp("empty");
        assert!(LocalDirectorySource::new(dir).scan().is_err());
    }

    #[test]
    fn forwards_parse_error_for_invalid_front_matter() {
        let dir = unique_tmp("bad");
        std::fs::write(dir.join("SKILL.md"), "no front matter here").unwrap();
        let err = LocalDirectorySource::new(dir).scan().expect_err("should fail");
        assert!(matches!(err, SourceError::Parse(_)));
    }
}
