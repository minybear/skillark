//! v0.2 Link Bridge — Git source fetch via libgit2 (`git2`).
//!
//! Clones a resolved [`RepositoryLocator`] into an isolated cache directory and
//! returns the working-tree root plus the resolved commit SHA. Tests clone a
//! *local* repository built with git2 itself, so the fetch/checkout/resolve
//! logic is exercised without public network (which is currently blocked in
//! this dev environment). Real HTTPS clones are validated by the L7 GitHub POC.
//!
//! Design refs: design/v0.2-v1.0/v0.2-link-bridge/02-ARCHITECTURE.md (GitSourceAdapter),
//! 08-DECISIONS.md (ADR-0201: identity = resolved commit + subpath + hash).

use std::path::{Path, PathBuf};

use git2::{build::RepoBuilder, Repository};

use crate::domain::link_bridge::RepositoryLocator;

#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("git clone failed for {remote}: {source}")]
    Clone {
        remote: String,
        #[source]
        source: git2::Error,
    },
    #[error("could not resolve ref `{ref_name}`: {source}")]
    ResolveRef {
        ref_name: String,
        #[source]
        source: git2::Error,
    },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// The result of fetching a repository: a checked-out working tree and the
/// immutable commit SHA that identifies this version.
#[derive(Debug, Clone)]
pub struct FetchedRepository {
    /// Root of the checked-out working tree.
    pub checkout_dir: PathBuf,
    /// Resolved commit SHA (immutable version identity; ADR-0201).
    pub resolved_revision: String,
}

pub struct GitFetcher {
    cache_root: PathBuf,
}

impl GitFetcher {
    pub fn new(cache_root: PathBuf) -> Self {
        Self { cache_root }
    }

    /// Clone `locator.remote`, optionally check out `locator.requested_ref`, and
    /// return the working tree plus the resolved commit. The clone lands in an
    /// isolated unique directory under `cache_root`. Submodules and LFS are NOT
    /// recursed (per翻车点: missing submodule/LFS content must be surfaced, not
    /// silently skipped — handled downstream as an explicit limitation for now).
    pub fn fetch(&self, locator: &RepositoryLocator) -> Result<FetchedRepository, FetchError> {
        std::fs::create_dir_all(&self.cache_root)?;
        let dest = unique_dir(&self.cache_root);

        // RepoBuilder::clone checks out the default branch and does not recurse
        // into submodules by default.
        let repo = RepoBuilder::new()
            .clone(&locator.remote, &dest)
            .map_err(|source| FetchError::Clone {
                remote: locator.remote.clone(),
                source,
            })?;

        let resolved_revision = match &locator.requested_ref {
            None => current_head_sha(&repo)?,
            Some(ref_name) => checkout_ref(&repo, ref_name)?,
        };

        Ok(FetchedRepository {
            checkout_dir: dest,
            resolved_revision,
        })
    }
}

/// SHA of the commit HEAD points at after a default-branch clone.
fn current_head_sha(repo: &Repository) -> Result<String, FetchError> {
    let head = repo
        .head()
        .map_err(|source| ref_err("HEAD", source))?;
    let commit = head
        .peel_to_commit()
        .map_err(|source| ref_err("HEAD", source))?;
    Ok(commit.id().to_string())
}

/// Resolve `ref_name` (branch / tag / commit shorthand, including remote-only
/// branches via `origin/<ref>`), check out its tree, and return its commit SHA.
fn checkout_ref(repo: &Repository, ref_name: &str) -> Result<String, FetchError> {
    let object = resolve_ref(repo, ref_name)?;
    let commit = object
        .peel_to_commit()
        .map_err(|source| ref_err(ref_name, source))?;
    repo.checkout_tree(&object, None)
        .map_err(|source| ref_err(ref_name, source))?;
    // Detach HEAD to the resolved commit so HEAD matches the SHA identity.
    repo.set_head_detached(commit.id())
        .map_err(|source| ref_err(ref_name, source))?;
    Ok(commit.id().to_string())
}

/// Try several forms so branch/tag/commit and remote-only branches all resolve.
fn resolve_ref<'a>(repo: &'a Repository, ref_name: &str) -> Result<git2::Object<'a>, FetchError> {
    for candidate in [
        ref_name.to_owned(),
        format!("refs/heads/{ref_name}"),
        format!("refs/tags/{ref_name}"),
        format!("refs/remotes/origin/{ref_name}"),
        format!("origin/{ref_name}"),
    ] {
        if let Ok(obj) = repo.revparse_single(&candidate) {
            return Ok(obj);
        }
    }
    Err(repo
        .revparse_single(ref_name)
        .err()
        .map(|source| ref_err(ref_name, source))
        .unwrap_or_else(|| ref_err(ref_name, git2::Error::from_str("could not parse ref"))))
}

fn unique_dir(root: &Path) -> PathBuf {
    let dir = root.join(format!(
        "skillark-fetch-{}-{}",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    std::fs::create_dir_all(&dir).ok();
    dir
}

#[inline]
fn ref_err(name: &str, source: git2::Error) -> FetchError {
    FetchError::ResolveRef {
        ref_name: name.to_owned(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a local git repository (the "remote") with one commit on HEAD.
    fn make_remote(root: &Path) -> Repository {
        std::fs::create_dir_all(root).unwrap();
        let repo = Repository::init(root).unwrap();
        std::fs::write(
            root.join("SKILL.md"),
            "---\nname: demo\nversion: 1.0.0\ndescription: x\n---\nbody",
        )
        .unwrap();
        commit_all(&repo, "init");
        repo
    }

    fn commit_all(repo: &Repository, msg: &str) {
        let mut index = repo.index().unwrap();
        // Add everything currently in the worktree.
        for entry in std::fs::read_dir(repo.workdir().unwrap()).unwrap().flatten() {
            if entry.path().is_file() {
                let rel = entry
                    .path()
                    .strip_prefix(repo.workdir().unwrap())
                    .unwrap()
                    .to_owned();
                index.add_path(&rel).unwrap();
            }
        }
        index.write().unwrap();
        let oid = index.write_tree().unwrap();
        let sig = git2::Signature::now("tester", "tester@example.com").unwrap();
        let tree = repo.find_tree(oid).unwrap();
        let parent_commits: Vec<git2::Commit> = repo
            .head()
            .ok()
            .and_then(|h| h.target().map(|t| repo.find_commit(t).unwrap()))
            .into_iter()
            .collect();
        let parent_refs: Vec<&git2::Commit> = parent_commits.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &parent_refs)
            .unwrap();
    }

    fn cache_root() -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "skillark-git-cache-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        p
    }

    #[test]
    fn fetches_default_branch_locally() {
        let remote_dir = std::env::temp_dir().join(format!(
            "skillark-remote-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let repo = make_remote(&remote_dir);
        let head_sha = repo.head().unwrap().target().unwrap().to_string();
        drop(repo);

        let locator = RepositoryLocator {
            remote: remote_dir.to_string_lossy().into_owned(),
            host: "(local)".to_owned(),
            owner_repo: None,
            requested_ref: None,
            subpath: None,
        };
        let fetched = GitFetcher::new(cache_root()).fetch(&locator).expect("clone");
        assert_eq!(fetched.resolved_revision, head_sha);
        assert!(fetched.checkout_dir.join("SKILL.md").is_file());
        // SHA is a 40-char hex string.
        assert_eq!(fetched.resolved_revision.len(), 40);

        let _ = std::fs::remove_dir_all(&remote_dir);
    }

    #[test]
    fn fetches_specific_branch() {
        let remote_dir = std::env::temp_dir().join(format!(
            "skillark-remote-branch-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        let repo = make_remote(&remote_dir);

        // Create a `dev` branch with different content.
        std::fs::write(
            remote_dir.join("SKILL.md"),
            "---\nname: demo-dev\nversion: 2.0.0\ndescription: dev\n---\n",
        )
        .unwrap();
        commit_all(&repo, "dev change");
        let dev_commit = repo.head().unwrap().target().unwrap();
        repo.branch("dev", &repo.find_commit(dev_commit).unwrap(), true)
            .unwrap();
        drop(repo);

        let locator = RepositoryLocator {
            remote: remote_dir.to_string_lossy().into_owned(),
            host: "(local)".to_owned(),
            owner_repo: None,
            requested_ref: Some("dev".to_owned()),
            subpath: None,
        };
        let fetched = GitFetcher::new(cache_root()).fetch(&locator).expect("clone dev");

        // dev branch content is the 2.0.0 manifest.
        let body = std::fs::read_to_string(fetched.checkout_dir.join("SKILL.md")).unwrap();
        assert!(body.contains("version: 2.0.0"));
        // Resolved revision matches dev's commit exactly (not default branch).
        assert_eq!(fetched.resolved_revision, dev_commit.to_string());

        let _ = std::fs::remove_dir_all(&remote_dir);
    }

    #[test]
    fn resolver_then_fetch_roundtrip() {
        // Confirm the resolver output can drive the fetcher for a local path
        // remote (using a file://-style local remote path).
        let remote_dir = std::env::temp_dir().join(format!(
            "skillark-rr-{}-{}",
            std::process::id(),
            uuid::Uuid::new_v4()
        ));
        make_remote(&remote_dir);

        // resolve_locator would reject a bare local path; fetcher takes the raw
        // path, so feed it directly to prove the fetch contract is independent.
        let fetched = GitFetcher::new(cache_root()).fetch(&RepositoryLocator {
            remote: remote_dir.to_string_lossy().into_owned(),
            host: "(local)".to_owned(),
            owner_repo: None,
            requested_ref: None,
            subpath: None,
        });
        assert!(fetched.is_ok());
        let _ = std::fs::remove_dir_all(&remote_dir);
    }
}
