//! Process-wide application state and shared path helpers.
//!
//! [`AppState`] is constructed once at startup (see `lib.rs`) and held in
//! Tauri's managed state. It owns the SQLite pool and the on-disk vault root.

use std::path::PathBuf;

use sqlx::SqlitePool;

use crate::{
    adapters::sqlite,
    repositories::{DeploymentRepository, OperationRepository, SkillRepository, WorkspaceRepository},
};

/// SkillArk's data root. Production defaults to `~/.skillark`; automated
/// desktop tests can set `SKILLARK_DATA_DIR` to keep real user data untouched.
pub fn skillark_dir() -> Result<PathBuf, String> {
    resolve_skillark_dir(
        std::env::var_os("SKILLARK_DATA_DIR"),
        std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")),
    )
}

pub fn home_dir() -> Result<PathBuf, String> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .ok_or_else(|| "The current user home directory could not be resolved.".to_owned())
}

fn resolve_skillark_dir(
    explicit: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
) -> Result<PathBuf, String> {
    if let Some(path) = explicit.filter(|value| !value.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    home.map(PathBuf::from)
        .map(|path| path.join(".skillark"))
        .ok_or_else(|| "The current user home directory could not be resolved.".to_owned())
}

/// Reduce a manifest `name` to a filesystem-safe canonical id.
///
/// Keeps `[A-Za-z0-9._-]`, replaces everything else with `_`, trims leading
/// separators, and falls back to `skill` for empty results.
pub fn sanitize_canonical(name: &str) -> String {
    let mapped: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = mapped
        .trim_matches(|c: char| c == '.' || c == '-' || c == '_')
        .to_owned();
    if trimmed.is_empty() {
        "skill".to_owned()
    } else {
        trimmed
    }
}

/// Short content-addressed suffix for a snapshot directory.
pub fn hash_prefix(content_hash: &str) -> &str {
    let len = content_hash.len().min(12);
    &content_hash[..len]
}

#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub vault_path: PathBuf,
}

impl AppState {
    /// Build the state: create `~/.skillark`, open+ migrate the database, seed
    /// the global workspace, and create the vault root.
    pub async fn setup() -> Result<Self, String> {
        let dir = skillark_dir()?;
        std::fs::create_dir_all(&dir).map_err(|e| format!("create {}: {e}", dir.display()))?;

        let db_path = dir.join("skillark.db");
        let url = format!("sqlite:{}", db_path.display());
        let pool = sqlx_connect(&url).await?;

        OperationRepository::new(pool.clone())
            .recover_interrupted()
            .await?;

        WorkspaceRepository::new(pool.clone())
            .ensure_global_default()
            .await?;

        let vault_path = dir.join("vault");
        std::fs::create_dir_all(&vault_path)
            .map_err(|e| format!("create vault {}: {e}", vault_path.display()))?;

        Ok(Self { pool, vault_path })
    }

    /// Construct against an already-open pool + vault (used by tests and the
    /// services layer).
    pub fn new(pool: SqlitePool, vault_path: PathBuf) -> Self {
        Self { pool, vault_path }
    }

    pub fn skills(&self) -> SkillRepository {
        SkillRepository::new(self.pool.clone())
    }
    pub fn operations(&self) -> OperationRepository {
        OperationRepository::new(self.pool.clone())
    }
    pub fn deployments(&self) -> DeploymentRepository {
        DeploymentRepository::new(self.pool.clone())
    }
    pub fn agents(&self) -> crate::repositories::AgentRepository {
        crate::repositories::AgentRepository::new(self.pool.clone())
    }
    pub fn workspaces(&self) -> WorkspaceRepository {
        WorkspaceRepository::new(self.pool.clone())
    }
}

async fn sqlx_connect(url: &str) -> Result<SqlitePool, String> {
    sqlite::connect(url).await.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_handles_spaces_unicode_and_dots() {
        assert_eq!(sanitize_canonical("My Cool Skill"), "My_Cool_Skill");
        assert_eq!(sanitize_canonical("技能 / 工具"), "skill");
        assert_eq!(sanitize_canonical("...leading"), "leading");
        assert_eq!(sanitize_canonical(""), "skill");
        assert_eq!(sanitize_canonical("ok.name-1_v2"), "ok.name-1_v2");
    }

    #[test]
    fn hash_prefix_is_twelve_chars() {
        assert_eq!(hash_prefix("abcdef0123456789"), "abcdef012345");
        assert_eq!(hash_prefix("short"), "short");
    }

    #[test]
    fn explicit_data_dir_overrides_home_default() {
        let explicit = std::ffi::OsString::from("D:/skillark-e2e");
        let home = std::ffi::OsString::from("C:/Users/example");
        assert_eq!(
            resolve_skillark_dir(Some(explicit), Some(home)).unwrap(),
            PathBuf::from("D:/skillark-e2e")
        );
    }
}
