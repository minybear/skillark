//! Read-only skill detail: the file tree of the current snapshot plus the
//! `SKILL.md` body, for the Library detail view (PRD §5.1).

use std::path::Path;

use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntryDto {
    pub path: String,
    pub size: u64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SkillDetailDto {
    pub id: String,
    pub canonical_name: String,
    pub display_name: String,
    pub description: String,
    pub content_hash: Option<String>,
    pub version_label: Option<String>,
    pub snapshot_path: String,
    pub skill_md: Option<String>,
    pub files: Vec<FileEntryDto>,
}

/// Walk `root` recording regular files (skipping symlinks / reparse points),
/// keyed by forward-slash relative path. Mirrors `content_hash`'s skip rules so
/// the listed files are exactly the ones that went into the hash.
pub fn list_files(root: &Path) -> Result<Vec<FileEntryDto>, String> {
    let mut out = Vec::new();
    walk(root, root, &mut out)?;
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

fn walk(root: &Path, current: &Path, out: &mut Vec<FileEntryDto>) -> Result<(), String> {
    let mut entries: Vec<_> = std::fs::read_dir(current)
        .map_err(|e| format!("read {}: {e}", current.display()))?
        .filter_map(|e| e.ok())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    for entry in entries {
        let path = entry.path();
        let meta = match std::fs::symlink_metadata(&path) {
            Ok(m) => m,
            Err(_) => continue,
        };
        if meta.file_type().is_symlink() {
            continue;
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if meta.file_attributes() & 0x400 != 0 {
                continue;
            }
        }
        if meta.is_dir() {
            walk(root, &path, out)?;
        } else if meta.is_file() {
            let rel = path
                .strip_prefix(root)
                .map_err(|e| format!("strip: {e}"))?
                .to_string_lossy()
                .replace('\\', "/");
            out.push(FileEntryDto {
                path: rel,
                size: meta.len(),
            });
        }
    }
    Ok(())
}

pub fn read_text(path: &Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_tmp(sub: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "skillark-detail-{}-{}-{sub}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn write(path: &std::path::Path, body: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, body).unwrap();
    }

    #[test]
    fn list_files_walks_and_sorts() {
        let dir = unique_tmp("walk");
        write(&dir.join("SKILL.md"), "---\nname: a\nversion: 1\n---\nbody");
        write(&dir.join("scripts/run.sh"), "echo");
        write(&dir.join("refs/deep.md"), "deep");

        let files = list_files(&dir).unwrap();
        let paths: Vec<_> = files.iter().map(|f| f.path.as_str()).collect();
        assert_eq!(paths, vec!["SKILL.md", "refs/deep.md", "scripts/run.sh"]);
        assert!(files.iter().any(|f| f.path == "scripts/run.sh" && f.size > 0));
    }

    #[test]
    fn read_text_returns_none_for_missing() {
        assert!(read_text(std::path::Path::new("/no/such/skillark-file")).is_none());
    }
}
