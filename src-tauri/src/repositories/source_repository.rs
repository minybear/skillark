//! Git source provenance repository (v0.2 Link Bridge).
//!
//! Backs the `sources` (reused from v0.1, generic) and `source_revisions`
//! (new in migration 0003) tables with runtime `sqlx::query`, matching the
//! v0.1 repository style. Records the immutable identity of a fetched Git
//! source: remote + resolved commit + subpath + content hash.

use sqlx::{Row, SqlitePool};
use uuid::Uuid;

pub struct SourceRepository {
    pool: SqlitePool,
}

impl SourceRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Find or create a `sources` row for a Git remote. `config_json` carries
    /// the canonical remote, requested ref and subpath (v0.1 sources is generic
    /// by design; Git specifics live in `config_json` + `source_revisions`).
    pub async fn find_or_create_git_source(
        &self,
        canonical_remote: &str,
        display_name: &str,
        config_json: &str,
    ) -> Result<Uuid, String> {
        if let Some(id) = self.find_source_by_base_url(canonical_remote).await? {
            return Ok(id);
        }
        let id = Uuid::new_v4();
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            r#"INSERT INTO sources (id, source_type, display_name, base_url, enabled, config_json, created_at, updated_at)
               VALUES (?, 'git', ?, ?, 1, ?, ?, ?)"#,
        )
        .bind(id.to_string())
        .bind(display_name)
        .bind(canonical_remote)
        .bind(config_json)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(id)
    }

    async fn find_source_by_base_url(&self, base_url: &str) -> Result<Option<Uuid>, String> {
        let row = sqlx::query(r#"SELECT id FROM sources WHERE base_url = ? AND source_type = 'git'"#)
            .bind(base_url)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(row.map(|r| Uuid::parse_str(&r.get::<String, _>("id")).expect("uuid")))
    }

    /// Record a fetch as an immutable source revision. Idempotent on
    /// (source_id, resolved_revision, subpath) via the unique index.
    pub async fn record_revision(
        &self,
        source_id: Uuid,
        resolved_revision: &str,
        requested_ref: Option<&str>,
        subpath: Option<&str>,
        content_hash: &str,
    ) -> Result<(), String> {
        // Insert-or-ignore keyed by the unique index; avoids duplicate rows on
        // repeated imports of the same link/revision.
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            r#"INSERT OR IGNORE INTO source_revisions
               (id, source_id, resolved_revision, requested_ref, subpath, content_hash, fetched_at, created_at)
               VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#,
        )
        .bind(Uuid::new_v4().to_string())
        .bind(source_id.to_string())
        .bind(resolved_revision)
        .bind(requested_ref.unwrap_or(""))
        .bind(subpath.unwrap_or(""))
        .bind(content_hash)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Point a skill at its Git source (skills.source_id) and stamp the version
    /// with the resolved commit (skill_versions.source_revision).
    pub async fn attach_provenance(
        &self,
        skill_id: Uuid,
        version_id: Uuid,
        source_id: Uuid,
        resolved_revision: &str,
    ) -> Result<(), String> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(r#"UPDATE skills SET source_id = ?, updated_at = ? WHERE id = ?"#)
            .bind(source_id.to_string())
            .bind(&now)
            .bind(skill_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        sqlx::query(r#"UPDATE skill_versions SET source_revision = ? WHERE id = ?"#)
            .bind(resolved_revision)
            .bind(version_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
