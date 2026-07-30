//! v0.2 Link Bridge — repository scanning + link import orchestration.
//!
//! L3 [`scan_repository`] walks a fetched checkout and enumerates every
//! `SKILL.md` as a [`SkillCandidate`] (multi-skill repos). L4
//! [`LinkImportService`] ties resolver → fetcher → scan → existing import path
//! together and records Git provenance.

use std::path::{Path, PathBuf};

use sqlx::SqlitePool;

use crate::{
    adapters::sources::{git::GitFetcher, LocalDirectorySource},
    application::import_skill::{ImportOutcome, ImportSkillService},
    domain::{
        link_bridge::{resolve_locator, RepositoryLocator},
        skill_manifest::parse_skill_md,
    },
    ports::SkillSourceAdapter,
    repositories::SourceRepository,
};

#[derive(Debug, thiserror::Error)]
pub enum ScanError {
    #[error("checkout directory does not exist: {0}")]
    Missing(PathBuf),
    #[error("hinted subpath not found in repository: {0}")]
    SubpathMissing(PathBuf),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// One discovered skill inside a fetched repository.
#[derive(Debug, Clone)]
pub struct SkillCandidate {
    /// Subpath within the repo (`""` for the root); becomes provenance.
    pub relative_path: String,
    /// Absolute skill root directory (the parent of its `SKILL.md`).
    pub source_root: PathBuf,
    /// Parsed manifest (name/version/description).
    pub manifest: crate::domain::skill_manifest::SkillManifest,
}

/// Walk `checkout_dir` and return every parseable `SKILL.md` as a candidate.
/// When `hint_subpath` is set, only that subdirectory is searched.
///
/// Symlinks/reparse points are skipped (never followed), mirroring the
/// hash/copy safety invariants: a fetched repo is untrusted external content.
pub fn scan_repository(
    checkout_dir: &Path,
    hint_subpath: Option<&str>,
) -> Result<Vec<SkillCandidate>, ScanError> {
    if !checkout_dir.is_dir() {
        return Err(ScanError::Missing(checkout_dir.to_path_buf()));
    }
    let search_root = match hint_subpath.filter(|p| !p.is_empty()) {
        Some(p) => {
            let dir = checkout_dir.join(p);
            if !dir.is_dir() {
                return Err(ScanError::SubpathMissing(dir));
            }
            dir
        }
        None => checkout_dir.to_path_buf(),
    };

    let mut found = Vec::new();
    collect_skills(&search_root, checkout_dir, &mut found)?;
    Ok(found)
}

fn collect_skills(
    dir: &Path,
    checkout_root: &Path,
    out: &mut Vec<SkillCandidate>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        // Skip symlinks / reparse points — fetched content is untrusted.
        let meta = std::fs::symlink_metadata(&path)?;
        if meta.file_type().is_symlink() {
            continue;
        }
        let name = entry.file_name();
        let name = name.to_string_lossy();

        if meta.is_dir() {
            // Never descend into git internals or heavy vendor dirs.
            if matches!(name.as_ref(), ".git" | "node_modules" | ".svn" | ".hg") {
                continue;
            }
            // A SKILL.md directly here means this dir is a skill root; do not
            // also descend looking for more (skills don't nest skills).
            if path.join("SKILL.md").is_file() {
                push_candidate(&path, checkout_root, out)?;
            } else {
                collect_skills(&path, checkout_root, out)?;
            }
        } else if meta.is_file() && name == "SKILL.md" {
            push_candidate(dir, checkout_root, out)?;
        }
    }
    Ok(())
}

fn push_candidate(
    skill_root: &Path,
    checkout_root: &Path,
    out: &mut Vec<SkillCandidate>,
) -> std::io::Result<()> {
    let skill_md = std::fs::read_to_string(skill_root.join("SKILL.md"))?;
    let manifest = match parse_skill_md(&skill_md) {
        Ok(m) => m,
        Err(_) => return Ok(()), // unparseable → not a valid skill, skip
    };
    let relative_path = skill_root
        .strip_prefix(checkout_root)
        .map(|p| p.to_string_lossy().replace('\\', "/"))
        .unwrap_or_default();
    out.push(SkillCandidate {
        relative_path,
        source_root: skill_root.to_path_buf(),
        manifest,
    });
    Ok(())
}

// ── L4: link import orchestration ───────────────────────────────────────

/// Preview a pasted link: resolve, fetch, scan — without importing. Returns the
/// resolved revision and the candidate skills the user can choose from.
pub struct LinkImportService {
    pool: SqlitePool,
    vault_path: PathBuf,
    fetch_cache_root: PathBuf,
}

impl LinkImportService {
    pub fn new(pool: SqlitePool, vault_path: PathBuf, fetch_cache_root: PathBuf) -> Self {
        Self {
            pool,
            vault_path,
            fetch_cache_root,
        }
    }

    /// Resolve + fetch + scan. The fetch lands in an isolated cache dir whose
    /// lifetime is managed by the caller via the returned [`LinkPreview`].
    pub async fn preview(&self, link: &str) -> Result<LinkPreview, String> {
        let locator = resolve_locator(link).map_err(|e| e.to_string())?;
        let fetcher = GitFetcher::new(self.fetch_cache_root.clone());
        let locator_for_fetch = locator.clone();
        let fetched = tauri::async_runtime::spawn_blocking(move || fetcher.fetch(&locator_for_fetch))
            .await
            .map_err(|e| format!("fetch task: {e}"))?
            .map_err(|e| e.to_string())?;

        let checkout_dir = fetched.checkout_dir.clone();
        let resolved_revision = fetched.resolved_revision.clone();
        let hint = locator.subpath.clone();
        let candidates = tauri::async_runtime::spawn_blocking(move || {
            scan_repository(&checkout_dir, hint.as_deref())
        })
        .await
        .map_err(|e| format!("scan task: {e}"))?
        .map_err(|e| e.to_string())?;

        Ok(LinkPreview {
            locator,
            resolved_revision,
            candidates,
            checkout_dir: fetched.checkout_dir,
        })
    }

    /// Import a chosen preview candidate into the Library, reusing the existing
    /// import path (hash → vault snapshot → skill + version) and then stamping
    /// Git provenance: a `sources` row, a `source_revisions` row, and the
    /// resolved commit on the skill version.
    pub async fn import_candidate(
        &self,
        preview: &LinkPreview,
        candidate_index: usize,
    ) -> Result<ImportOutcome, String> {
        let candidate = preview
            .candidates
            .get(candidate_index)
            .ok_or_else(|| "invalid candidate index".to_string())?;

        // 1. Scan the chosen skill dir via the existing adapter → ScannedSource.
        let scanned = LocalDirectorySource::new(candidate.source_root.clone())
            .scan()
            .map_err(|e| e.to_string())?;

        // 2. Import (content hash, immutable vault snapshot, dedup skill+version).
        let importer = ImportSkillService::new(self.vault_path.clone(), self.pool.clone());
        let outcome = importer.import_scanned(scanned).await?;

        // 3. Record Git provenance.
        let config_json = serde_json::json!({
            "remote": preview.locator.remote,
            "requestedRef": preview.locator.requested_ref,
            "hintSubpath": preview.locator.subpath,
        })
        .to_string();
        let sources = SourceRepository::new(self.pool.clone());
        let source_id = sources
            .find_or_create_git_source(
                &preview.locator.remote,
                &preview.locator.display_label(),
                &config_json,
            )
            .await?;
        let subpath = if candidate.relative_path.is_empty() {
            None
        } else {
            Some(candidate.relative_path.as_str())
        };
        sources
            .record_revision(
                source_id,
                &preview.resolved_revision,
                preview.locator.requested_ref.as_deref(),
                subpath,
                &outcome.content_hash,
            )
            .await?;
        sources
            .attach_provenance(
                outcome.skill_id,
                outcome.version_id,
                source_id,
                &preview.resolved_revision,
            )
            .await?;

        Ok(outcome)
    }
}

/// Outcome of previewing a link: what was fetched and which skills were found.
#[derive(Debug, Clone)]
pub struct LinkPreview {
    /// The resolved source locator (remote / ref / subpath) — carried for
    /// provenance when the user imports a candidate.
    pub locator: RepositoryLocator,
    pub resolved_revision: String,
    pub candidates: Vec<SkillCandidate>,
    /// The fetched working tree (kept alive while the user decides).
    pub checkout_dir: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_skill(dir: &Path, name: &str, version: &str) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(
            dir.join("SKILL.md"),
            format!("---\nname: {name}\nversion: {version}\ndescription: d\n---\nbody"),
        )
        .unwrap();
    }

    fn unique_repo(sub: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!(
            "skillark-scan-{sub}-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn finds_single_skill_at_root() {
        let repo = unique_repo("root");
        write_skill(&repo, "alpha", "1.0.0");
        let cands = scan_repository(&repo, None).unwrap();
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0].manifest.name, "alpha");
        assert_eq!(cands[0].relative_path, "");
    }

    #[test]
    fn finds_multiple_skills_in_subdirs() {
        let repo = unique_repo("multi");
        write_skill(&repo.join("skills/one"), "one", "1.0.0");
        write_skill(&repo.join("skills/two"), "two", "2.0.0");
        // an unrelated README at root must not be a candidate
        std::fs::write(repo.join("README.md"), "nope").unwrap();
        let cands = scan_repository(&repo, None).unwrap();
        let names: Vec<_> = cands.iter().map(|c| c.manifest.name.as_str()).collect();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"one"));
        assert!(names.contains(&"two"));
        let rels: Vec<_> = cands.iter().map(|c| c.relative_path.as_str()).collect();
        assert!(rels.contains(&"skills/one"));
        assert!(rels.contains(&"skills/two"));
    }

    #[test]
    fn scoped_to_hint_subpath() {
        let repo = unique_repo("hint");
        write_skill(&repo.join("skills/a"), "a", "1.0.0");
        write_skill(&repo.join("skills/b"), "b", "1.0.0");
        let cands = scan_repository(&repo, Some("skills/a")).unwrap();
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0].manifest.name, "a");
    }

    #[test]
    fn ignores_git_internals() {
        let repo = unique_repo("gitdir");
        write_skill(&repo, "real", "1.0.0");
        // fake .git internals with a stray SKILL.md — must be ignored
        std::fs::create_dir_all(repo.join(".git/hooks")).unwrap();
        std::fs::write(repo.join(".git/hooks/SKILL.md"), "---\nname: fake\nversion: 1\n---\n").unwrap();
        let cands = scan_repository(&repo, None).unwrap();
        assert_eq!(cands.len(), 1);
        assert_eq!(cands[0].manifest.name, "real");
    }

    #[test]
    fn empty_repo_yields_no_candidates() {
        let repo = unique_repo("empty");
        let cands = scan_repository(&repo, None).unwrap();
        assert!(cands.is_empty());
    }

    #[test]
    fn reuses_local_directory_source_for_chosen_candidate() {
        // After scanning, a chosen candidate's source_root is a plain dir that
        // LocalDirectorySource can scan — proving the bridge into import_scanned.
        let repo = unique_repo("bridge");
        write_skill(&repo, "bridged", "1.0.0");
        let cands = scan_repository(&repo, None).unwrap();
        let chosen = &cands[0];
        let scanned = LocalDirectorySource::new(chosen.source_root.clone())
            .scan()
            .expect("scans via existing adapter");
        assert_eq!(scanned.manifest.name, "bridged");
    }
}
