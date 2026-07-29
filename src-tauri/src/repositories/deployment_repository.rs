//! Deployment record repository.
//!
//! Mirrors the `deployments` table. The partial unique index
//! `idx_deployments_target_path WHERE status != 'uninstalled'` means at most one
//! active deployment per target path; [`DeploymentRepository::upsert_active`]
//! preserves that invariant by retiring any prior active row first.

use std::path::PathBuf;
use std::str::FromStr;

use sqlx::{Row, SqlitePool};

use crate::domain::deployment::{
    DeploymentRecord, DeploymentStatus, InstallMode,
};

pub struct DeploymentRepository {
    pool: SqlitePool,
}

impl DeploymentRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Insert (or replace) the active deployment for a target path.
    pub async fn upsert_active(&self, record: &DeploymentRecord) -> Result<(), String> {
        let now = chrono::Utc::now().to_rfc3339();
        // Retire any prior active deployment at this path to honour the partial
        // unique index, then insert the fresh record.
        sqlx::query(
            r#"DELETE FROM deployments WHERE target_path = ? AND status != 'uninstalled'"#,
        )
        .bind(record.target_path.to_string_lossy().to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        sqlx::query(
            r#"INSERT INTO deployments (
                id, skill_version_id, agent_id, workspace_id, operation_id,
                target_path, install_mode, status, deployed_hash,
                source_path_at_install, installed_at, last_verified_at,
                error_message, created_at, updated_at
              ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL, ?, ?, ?)"#,
        )
        .bind(&record.id)
        .bind(&record.skill_version_id)
        .bind(&record.agent_id)
        .bind(&record.workspace_id)
        .bind(&record.operation_id)
        .bind(record.target_path.to_string_lossy().to_string())
        .bind(record.install_mode.as_str())
        .bind(record.status.as_str())
        .bind(&record.deployed_hash)
        .bind(record.source_path_at_install.to_string_lossy().to_string())
        .bind(&record.installed_at)
        .bind(&record.error_message)
        .bind(&record.created_at)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn find_active_by_target(
        &self,
        target_path: &std::path::Path,
    ) -> Result<Option<DeploymentRecord>, String> {
        let row = sqlx::query(
            r#"SELECT id, skill_version_id, agent_id, workspace_id, operation_id,
                      target_path, install_mode, status, deployed_hash,
                      source_path_at_install, installed_at, last_verified_at,
                      error_message, created_at, updated_at
               FROM deployments
               WHERE target_path = ? AND status != 'uninstalled'"#,
        )
        .bind(target_path.to_string_lossy().to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(row.map(|r| Self::row_to_record(&r)))
    }

    pub async fn get(&self, id: &str) -> Result<Option<DeploymentRecord>, String> {
        let row = sqlx::query(
            r#"SELECT id, skill_version_id, agent_id, workspace_id, operation_id,
                      target_path, install_mode, status, deployed_hash,
                      source_path_at_install, installed_at, last_verified_at,
                      error_message, created_at, updated_at
               FROM deployments WHERE id = ?"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.map(|r| Self::row_to_record(&r)))
    }

    pub async fn list_by_skill_version(
        &self,
        skill_version_id: &str,
    ) -> Result<Vec<DeploymentRecord>, String> {
        self.list_where(
            "skill_version_id = ? AND status != 'uninstalled'",
            vec![skill_version_id.to_owned()],
        )
        .await
    }

    pub async fn list_active(&self) -> Result<Vec<DeploymentRecord>, String> {
        self.list_where("status != 'uninstalled'", vec![]).await
    }

    async fn list_where(
        &self,
        clause: &str,
        binds: Vec<String>,
    ) -> Result<Vec<DeploymentRecord>, String> {
        let sql = format!(
            r#"SELECT id, skill_version_id, agent_id, workspace_id, operation_id,
                      target_path, install_mode, status, deployed_hash,
                      source_path_at_install, installed_at, last_verified_at,
                      error_message, created_at, updated_at
               FROM deployments WHERE {clause} ORDER BY updated_at DESC"#
        );
        let mut q = sqlx::query(&sql);
        for b in &binds {
            q = q.bind(b);
        }
        let rows = q.fetch_all(&self.pool).await.map_err(|e| e.to_string())?;
        Ok(rows.iter().map(Self::row_to_record).collect())
    }

    pub async fn set_status(
        &self,
        id: &str,
        status: DeploymentStatus,
        error_message: Option<&str>,
    ) -> Result<(), String> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            r#"UPDATE deployments
               SET status = ?, error_message = ?, updated_at = ?
               WHERE id = ?"#,
        )
        .bind(status.as_str())
        .bind(error_message)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn mark_verified(&self, id: &str, last_verified_at: &str) -> Result<(), String> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            r#"UPDATE deployments
               SET last_verified_at = ?, updated_at = ?
               WHERE id = ?"#,
        )
        .bind(last_verified_at)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    fn row_to_record(row: &sqlx::sqlite::SqliteRow) -> DeploymentRecord {
        let mode_str: &str = row.get("install_mode");
        let status_str: &str = row.get("status");
        let target: String = row.get("target_path");
        let source: String = row.get("source_path_at_install");
        DeploymentRecord {
            id: row.get("id"),
            skill_version_id: row.get("skill_version_id"),
            agent_id: row.get("agent_id"),
            workspace_id: row.get("workspace_id"),
            operation_id: row.get("operation_id"),
            target_path: PathBuf::from(target),
            install_mode: InstallMode::from_str(mode_str).unwrap_or(InstallMode::Copy),
            status: DeploymentStatus::from_str(status_str).unwrap_or(DeploymentStatus::Failed),
            deployed_hash: row.get("deployed_hash"),
            source_path_at_install: PathBuf::from(source),
            installed_at: row.get("installed_at"),
            last_verified_at: row.get("last_verified_at"),
            error_message: row.get("error_message"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }
}
