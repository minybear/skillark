//! Workspace repository — the fixed `global-default` row plus project workspaces.

use std::path::PathBuf;

use sqlx::{Row, SqlitePool};

use crate::domain::workspace::{Workspace, WorkspaceId, WorkspaceKind};

/// The stable id of the always-present user-level workspace (ADR-009).
pub const GLOBAL_DEFAULT_ID: &str = "global-default";

pub struct WorkspaceRepository {
    pool: SqlitePool,
}

impl WorkspaceRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Insert the fixed global workspace if it does not yet exist. Idempotent.
    pub async fn ensure_global_default(&self) -> Result<(), String> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            r#"INSERT OR IGNORE INTO workspaces
               (id, workspace_type, name, root_path, status, created_at, updated_at)
               VALUES (?, 'global', 'Global', NULL, 'available', ?, ?)"#,
        )
        .bind(GLOBAL_DEFAULT_ID)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn create_project(
        &self,
        id: &str,
        name: &str,
        root_path: Option<&std::path::Path>,
    ) -> Result<(), String> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            r#"INSERT INTO workspaces
               (id, workspace_type, name, root_path, status, created_at, updated_at)
               VALUES (?, 'project', ?, ?, 'available', ?, ?)"#,
        )
        .bind(id)
        .bind(name)
        .bind(root_path.map(|p| p.to_string_lossy().into_owned()))
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn list(&self) -> Result<Vec<Workspace>, String> {
        let rows = sqlx::query(
            r#"SELECT id, workspace_type, name, root_path FROM workspaces ORDER BY workspace_type, name"#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(rows.iter().map(Self::row_to_workspace).collect())
    }

    pub async fn get(&self, id: &str) -> Result<Option<Workspace>, String> {
        let row = sqlx::query(
            r#"SELECT id, workspace_type, name, root_path FROM workspaces WHERE id = ?"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(row.map(|r| Self::row_to_workspace(&r)))
    }

    /// Delete a workspace. The fixed `global-default` row is protected.
    pub async fn delete(&self, id: &str) -> Result<(), String> {
        if id == GLOBAL_DEFAULT_ID {
            return Err("the global workspace cannot be deleted".to_owned());
        }
        sqlx::query(r#"DELETE FROM workspaces WHERE id = ?"#)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn row_to_workspace(row: &sqlx::sqlite::SqliteRow) -> Workspace {
        let kind_str: &str = row.get("workspace_type");
        let kind = match kind_str {
            "project" => WorkspaceKind::Project,
            _ => WorkspaceKind::Global,
        };
        let root_path: Option<String> = row.get("root_path");
        Workspace {
            id: WorkspaceId(row.get("id")),
            kind,
            name: row.get("name"),
            root_path: root_path.map(PathBuf::from),
        }
    }
}
