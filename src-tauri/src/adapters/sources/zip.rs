//! ZIP skill source.
//!
//! Extracts an archive into a caller-provided scratch directory with Zip-Slip
//! protection, then the import service scans the located root via the
//! local-directory adapter.
//!
//! Zip-Slip defense: every entry name must
//!   1. survive `zip`'s own [`enclosed_name`](zip::read::ZipFile::enclosed_name)
//!      sanitization (drops leading `/`, rejects `..`), and
//!   2. pass [`path_safety::has_no_traversal`](crate::domain::path_safety::has_no_traversal)
//!      as a second, independent check.
//!
//! Any entry that fails either guard aborts the whole extraction so no partial,
//! escaped file ever lands on disk.

use std::{fs::File, io, path::Path};

use zip::ZipArchive;

use crate::domain::path_safety;

use super::SourceError;

pub struct ZipSource {
    zip_path: std::path::PathBuf,
}

impl ZipSource {
    pub fn new(zip_path: std::path::PathBuf) -> Self {
        Self { zip_path }
    }

    /// Extract the archive into `dest`. Rejects traversal entries; on any
    /// rejection or I/O failure, `dest` is best-effort cleaned up.
    pub fn extract_to(&self, dest: &Path) -> Result<(), SourceError> {
        let file = File::open(&self.zip_path)?;
        let mut archive = ZipArchive::new(file)
            .map_err(|e| SourceError::Zip(format!("open archive: {e}")))?;

        for index in 0..archive.len() {
            let mut entry = archive
                .by_index(index)
                .map_err(|e| SourceError::Zip(format!("read entry {index}: {e}")))?;

            // Primary guard: the zip crate's own sanitizer.
            let rel = match entry.enclosed_name() {
                Some(p) => p,
                None => {
                    return Err(self.abort(dest, SourceError::ZipSlip(entry.name().to_owned())));
                }
            };
            // Secondary independent guard: reject any literal `..` component.
            if !path_safety::has_no_traversal(&rel) {
                return Err(self.abort(dest, SourceError::ZipSlip(entry.name().to_owned())));
            }

            let out = dest.join(&rel);
            // `out` must stay inside `dest` by lexical containment.
            if !is_lexically_within(dest, &out) {
                return Err(self.abort(dest, SourceError::ZipSlip(entry.name().to_owned())));
            }

            if entry.is_dir() {
                std::fs::create_dir_all(&out)?;
            } else {
                if let Some(parent) = out.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut out_file = File::create(&out)?;
                io::copy(&mut entry, &mut out_file)?;
            }
        }
        Ok(())
    }

    fn abort(&self, dest: &Path, err: SourceError) -> SourceError {
        let _ = crate::adapters::filesystem::force_remove_dir_all(dest);
        err
    }
}

/// Pure containment check without touching the filesystem: `path` must equal
/// `root` or sit beneath it, after collapsing `.` components.
fn is_lexically_within(root: &Path, path: &Path) -> bool {
    let strip = |p: &Path| -> Vec<String> {
        p.components()
            .filter_map(|c| match c {
                std::path::Component::Normal(s) => Some(s.to_string_lossy().into_owned()),
                std::path::Component::RootDir => None,
                std::path::Component::Prefix(_) => None,
                _ => None,
            })
            .collect()
    };
    let root_parts = strip(root);
    let path_parts = strip(path);
    path_parts.len() >= root_parts.len()
        && path_parts[..root_parts.len()] == root_parts[..]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use zip::{write::SimpleFileOptions, ZipWriter};

    fn unique_tmp(sub: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "skillark-zip-{}-{}-{sub}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn make_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let mut zw = ZipWriter::new(File::create(path).unwrap());
        let opts = SimpleFileOptions::default();
        for (name, body) in entries {
            if name.ends_with('/') {
                zw.add_directory(*name, opts).unwrap();
            } else {
                zw.start_file(*name, opts).unwrap();
                zw.write_all(body).unwrap();
            }
        }
        zw.finish().unwrap();
    }

    #[test]
    fn extracts_valid_zip() {
        let dir = unique_tmp("valid");
        let zip_path = dir.join("s.zip");
        make_zip(
            &zip_path,
            &[
                ("skill/SKILL.md", b"---\nname: a\nversion: 1\n---\n"),
                ("skill/scripts/run.sh", b"echo hi"),
            ],
        );
        let dest = unique_tmp("valid-out");
        ZipSource::new(zip_path).extract_to(&dest).expect("extracts");

        assert!(dest.join("skill/SKILL.md").is_file());
        assert_eq!(
            std::fs::read_to_string(dest.join("skill/scripts/run.sh")).unwrap(),
            "echo hi"
        );
    }

    #[test]
    fn rejects_zip_slip_entry() {
        let dir = unique_tmp("slip");
        let zip_path = dir.join("malicious.zip");
        // A malicious entry that climbs out of the extract root.
        make_zip(&zip_path, &[("../escaped.txt", b"pwned")]);

        let dest = unique_tmp("slip-out");
        let err = ZipSource::new(zip_path).extract_to(&dest).expect_err("must reject");
        assert!(matches!(err, SourceError::ZipSlip(_)), "got {err:?}");
        // And the escape must not have happened.
        assert!(
            !dir.parent().unwrap().join("escaped.txt").exists(),
            "escaped file must not be written"
        );
    }

    #[test]
    fn rejects_absolute_entry() {
        let dir = unique_tmp("abs");
        let zip_path = dir.join("abs.zip");
        make_zip(&zip_path, &[("/etc/absolute.txt", b"x")]);

        let dest = unique_tmp("abs-out");
        let err = ZipSource::new(zip_path).extract_to(&dest).expect_err("must reject");
        assert!(matches!(err, SourceError::ZipSlip(_)), "got {err:?}");
    }
}
