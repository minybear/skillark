//! v0.2 Link Bridge — link resolution (pure, no IO, no network).
//!
//! Turns a user-pasted Git URL (GitHub repo / subdirectory / SSH / arbitrary git
//! URL) into a [`RepositoryLocator`]: a canonical remote plus an optional
//! requested ref and subpath. The resolver never touches the network or the
//! database — it only normalizes and validates. See `plan/20260730-v0.2-link-bridge`.
//!
//! Design refs: design/v0.2-v1.0/v0.2-link-bridge/01-PRD.md (H-02-01),
//! 08-DECISIONS.md (ADR-0201: identity = resolved commit + subpath + hash;
//! ADR-0203: parse failure returns an actionable error, never free-form guess).

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryLocator {
    /// Canonical remote URL to clone/fetch (HTTPS, `.git`-suffixed for GitHub).
    pub remote: String,
    /// Host without scheme, e.g. `github.com`.
    pub host: String,
    /// `owner/repo` for GitHub-style hosts; `None` for hosts we don't model.
    pub owner_repo: Option<String>,
    /// Branch / tag / commit the user asked for, or `None` for default branch.
    pub requested_ref: Option<String>,
    /// Repository-relative subdirectory (`skills/foo`); `None` means whole repo.
    pub subpath: Option<String>,
}

impl RepositoryLocator {
    /// A short, human-readable label (owner/repo or host/path).
    pub fn display_label(&self) -> String {
        match &self.owner_repo {
            Some(or) => or.clone(),
            None => {
                let trimmed = self.remote.trim_end_matches(".git");
                trimmed
                    .strip_prefix("https://")
                    .or_else(|| trimmed.strip_prefix("http://"))
                    .unwrap_or(trimmed)
                    .to_owned()
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ResolverError {
    #[error("link is empty")]
    Empty,
    #[error("unsupported link form (expected a git/http(s) URL): {0}")]
    UnsupportedScheme(String),
    #[error("repository path is incomplete (need host/owner/repo): {0}")]
    IncompletePath(String),
    #[error("subpath escapes the repository root (`..` or absolute not allowed): {0}")]
    SubpathTraversal(String),
}

/// Resolve a pasted link into a [`RepositoryLocator`].
///
/// Handles, for `github.com`:
/// - `https://github.com/OWNER/REPO`
/// - `https://github.com/OWNER/REPO.git`
/// - `https://github.com/OWNER/REPO/tree/REF` / `/tree/REF/SUBDIR`
/// - `https://github.com/OWNER/REPO/blob/REF/...` (ref captured, path ignored)
/// - `git@github.com:OWNER/REPO.git`
///
/// For other hosts (gitlab/bitbucket/arbitrary): only the canonical remote is
/// produced; ref/subpath stay `None` (the repo is fetched at its default branch
/// and scanned wholesale). Anything that cannot be normalized confidently is
/// rejected with an actionable error rather than guessed.
pub fn resolve_locator(input: &str) -> Result<RepositoryLocator, ResolverError> {
    let raw = input.trim();
    if raw.is_empty() {
        return Err(ResolverError::Empty);
    }
    // Refuse obvious local filesystem paths — these are not links.
    if looks_like_local_path(raw) {
        return Err(ResolverError::UnsupportedScheme(raw.to_owned()));
    }

    // Normalize SSH `git@host:owner/repo.git` → https form for parsing.
    let https_input = match parse_ssh(raw) {
        Some(normalized) => normalized,
        None => raw.to_owned(),
    };

    let rest = https_input
        .strip_prefix("https://")
        .or_else(|| https_input.strip_prefix("http://"))
        .ok_or_else(|| ResolverError::UnsupportedScheme(raw.to_owned()))?;

    // Split host from the path portion (drop query/fragment — they are not part
    // of a repo identity).
    let (host, mut path) = match rest.split_once('/') {
        Some((h, p)) => (h.trim_end_matches(':').to_owned(), p.to_owned()),
        None => return Err(ResolverError::IncompletePath(raw.to_owned())),
    };
    if let Some(idx) = path.find(['?', '#']) {
        path.truncate(idx);
    }
    let path = path.trim_end_matches('/');

    if host.eq_ignore_ascii_case("github.com") {
        return resolve_github(&host, path, raw);
    }

    // Non-GitHub host: keep the canonical remote only. Normalize a trailing
    // `.git` consistently off the path for the remote but keep it in the URL.
    let remote = format!("https://{host}/{path}");
    if path.is_empty() {
        return Err(ResolverError::IncompletePath(raw.to_owned()));
    }
    Ok(RepositoryLocator {
        remote,
        host,
        owner_repo: None,
        requested_ref: None,
        subpath: None,
    })
}

/// GitHub-specific resolution: extract owner/repo, optional ref and subpath.
fn resolve_github(host: &str, path: &str, raw: &str) -> Result<RepositoryLocator, ResolverError> {
    // path looks like: OWNER/REPO(.git)?(/tree|blob/REF(/SUBPATH)?)?
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if segments.len() < 2 {
        return Err(ResolverError::IncompletePath(raw.to_owned()));
    }

    let owner = segments[0].to_owned();
    let repo = segments[1].trim_end_matches(".git").to_owned();
    if owner.is_empty() || repo.is_empty() {
        return Err(ResolverError::IncompletePath(raw.to_owned()));
    }
    let owner_repo = format!("{owner}/{repo}");
    let remote = format!("https://{host}/{owner}/{repo}.git");

    // Anything beyond OWNER/REPO must start with `tree` or `blob`.
    let extra = &segments[2..];
    let (requested_ref, subpath) = parse_github_extra(extra, raw)?;

    Ok(RepositoryLocator {
        remote,
        host: host.to_owned(),
        owner_repo: Some(owner_repo),
        requested_ref,
        subpath,
    })
}

/// Parse the `/tree/REF[/SUB...]` or `/blob/REF[/...]` tail of a GitHub URL.
fn parse_github_extra(extra: &[&str], raw: &str) -> Result<(Option<String>, Option<String>), ResolverError> {
    if extra.is_empty() {
        return Ok((None, None));
    }
    let kind = extra[0].to_ascii_lowercase();
    if kind != "tree" && kind != "blob" {
        // Unknown tail (e.g. /settings, /pulls): ignore it, fetch default branch.
        return Ok((None, None));
    }
    if extra.len() < 2 {
        // `/tree` with no ref — treat as default branch, no subpath.
        return Ok((None, None));
    }
    let requested_ref = Some(extra[1].to_owned());

    let subpath = if kind == "tree" && extra.len() > 2 {
        let joined = extra[2..].join("/");
        Some(validate_subpath(&joined, raw)?)
    } else {
        // blob, or tree with no subpath
        None
    };

    Ok((requested_ref, subpath))
}

/// Reject subpaths that could escape the repository root.
fn validate_subpath(subpath: &str, raw: &str) -> Result<String, ResolverError> {
    if subpath.is_empty() {
        return Ok(subpath.to_owned());
    }
    if subpath.contains('\\') || subpath.starts_with('/') {
        return Err(ResolverError::SubpathTraversal(raw.to_owned()));
    }
    for component in subpath.split('/') {
        if component == ".." || component.is_empty() {
            return Err(ResolverError::SubpathTraversal(raw.to_owned()));
        }
    }
    Ok(subpath.to_owned())
}

/// `git@host:owner/repo.git` → `https://host/owner/repo.git`. Returns None if
/// the input is not the SSH scp-like form.
fn parse_ssh(input: &str) -> Option<String> {
    let s = input.trim();
    let at = s.find('@')?;
    let colon = s.find(':')?;
    if colon < at {
        return None;
    }
    // Must look like `git@github.com:owner/repo...` (no slashes before colon).
    let user_host = &s[at + 1..colon];
    if user_host.is_empty() || user_host.contains('/') {
        return None;
    }
    let path = &s[colon + 1..];
    if path.is_empty() {
        return None;
    }
    Some(format!("https://{user_host}/{path}"))
}

fn looks_like_local_path(s: &str) -> bool {
    // Windows drive path, UNC, or a bare relative path with no scheme/host.
    let t = s.trim();
    if t.starts_with('\\') {
        return true;
    }
    if t.len() >= 2 && t.as_bytes()[1] == b':' && (t.as_bytes()[0].is_ascii_alphabetic()) {
        return true;
    }
    // Bare relative path like "foo/bar" or "./x" — no scheme, no host dot.
    !t.contains("://") && !t.contains('@') && !t.contains('.')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(remote: &str, owner_repo: &str, requested_ref: Option<&str>, subpath: Option<&str>) -> RepositoryLocator {
        RepositoryLocator {
            remote: remote.to_owned(),
            host: "github.com".to_owned(),
            owner_repo: Some(owner_repo.to_owned()),
            requested_ref: requested_ref.map(str::to_owned),
            subpath: subpath.map(str::to_owned),
        }
    }

    #[test]
    fn https_repo_bare() {
        let l = resolve_locator("https://github.com/octocat/Hello-World").unwrap();
        assert_eq!(l, r("https://github.com/octocat/Hello-World.git", "octocat/Hello-World", None, None));
    }

    #[test]
    fn https_repo_dot_git_and_trailing_slash() {
        let l = resolve_locator("https://github.com/octocat/Hello-World.git/").unwrap();
        assert_eq!(l.remote, "https://github.com/octocat/Hello-World.git");
        assert_eq!(l.owner_repo.as_deref(), Some("octocat/Hello-World"));
    }

    #[test]
    fn https_tree_default_branch_no_subpath() {
        let l = resolve_locator("https://github.com/octocat/Hello-World/tree/main").unwrap();
        assert_eq!(l.requested_ref.as_deref(), Some("main"));
        assert!(l.subpath.is_none());
    }

    #[test]
    fn https_tree_branch_with_subpath() {
        let l = resolve_locator("https://github.com/octocat/Hello-World/tree/main/skills/foo").unwrap();
        assert_eq!(l.requested_ref.as_deref(), Some("main"));
        assert_eq!(l.subpath.as_deref(), Some("skills/foo"));
    }

    #[test]
    fn https_tree_commit_with_subpath() {
        let l = resolve_locator("https://github.com/o/r/tree/abc123def/skills/bar").unwrap();
        assert_eq!(l.requested_ref.as_deref(), Some("abc123def"));
        assert_eq!(l.subpath.as_deref(), Some("skills/bar"));
    }

    #[test]
    fn https_blob_captures_ref_ignores_path() {
        let l = resolve_locator("https://github.com/o/r/blob/main/README.md").unwrap();
        assert_eq!(l.requested_ref.as_deref(), Some("main"));
        assert!(l.subpath.is_none());
    }

    #[test]
    fn ssh_form() {
        let l = resolve_locator("git@github.com:octocat/Hello-World.git").unwrap();
        assert_eq!(l.remote, "https://github.com/octocat/Hello-World.git");
        assert_eq!(l.owner_repo.as_deref(), Some("octocat/Hello-World"));
    }

    #[test]
    fn query_and_fragment_dropped() {
        let l = resolve_locator("https://github.com/o/r?tab=readme").unwrap();
        assert_eq!(l.owner_repo.as_deref(), Some("o/r"));
        let l2 = resolve_locator("https://github.com/o/r/tree/main/x#readme").unwrap();
        assert_eq!(l2.subpath.as_deref(), Some("x"));
    }

    #[test]
    fn non_github_host_keeps_remote_only() {
        let l = resolve_locator("https://gitlab.com/group/proj.git").unwrap();
        assert_eq!(l.host, "gitlab.com");
        assert_eq!(l.remote, "https://gitlab.com/group/proj.git");
        assert!(l.owner_repo.is_none());
        assert!(l.requested_ref.is_none());
        assert!(l.subpath.is_none());
    }

    // ── security: 0 path-traversal ────────────────────────────────────

    #[test]
    fn rejects_dotdot_in_subpath() {
        assert!(matches!(
            resolve_locator("https://github.com/o/r/tree/main/.."),
            Err(ResolverError::SubpathTraversal(_))
        ));
        assert!(matches!(
            resolve_locator("https://github.com/o/r/tree/main/a/../../b"),
            Err(ResolverError::SubpathTraversal(_))
        ));
    }

    #[test]
    fn rejects_absolute_or_backslash_subpath() {
        // backslash component rejected by validate_subpath
        assert!(resolve_locator("https://github.com/o/r/tree/main/a\\b").is_err());
    }

    #[test]
    fn rejects_local_paths() {
        assert!(resolve_locator("C:\\Users\\me\\skill").is_err());
        assert!(resolve_locator("\\\\server\\share").is_err());
    }

    #[test]
    fn rejects_empty_and_bare_relative() {
        assert!(matches!(resolve_locator("   "), Err(ResolverError::Empty)));
        assert!(resolve_locator("foo/bar").is_err()); // no scheme/host dot
    }

    #[test]
    fn display_label_for_github_and_other() {
        let g = resolve_locator("https://github.com/o/r").unwrap();
        assert_eq!(g.display_label(), "o/r");
        let other = resolve_locator("https://gitlab.com/group/proj.git").unwrap();
        assert_eq!(other.display_label(), "gitlab.com/group/proj");
    }
}
