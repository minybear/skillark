//! Skill persistence repository backed by SQLite.
//!
//! Uses raw `sqlx::query` + manual Row→struct mapping so tests can run against
//! an in-memory database without needing compile-time table resolution from
//! `sqlx::query_as!`.

use sqlx::{SqlitePool, Row};
use uuid::Uuid;

use crate::domain::skill::{Skill, SkillStatus, SkillVersion};

pub struct SkillRepository {
    pool: SqlitePool,
}

impl SkillRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    // ── Skill CRUD ────────────────────────────────────────────────

    pub async fn create_skill(&self, skill: &Skill) -> Result<(), String> {
        sqlx::query(
            r#"INSERT INTO skills (
                id, canonical_name, display_name, description, format,
                library_path, status, created_at, updated_at
              ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"#
        )
        .bind(skill.id.to_string())
        .bind(&skill.canonical_name)
        .bind(&skill.display_name)
        .bind(&skill.description)
        .bind(&skill.format)
        .bind(&skill.library_path)
        .bind(skill.status.to_string())
        .bind(&skill.created_at)
        .bind(&skill.updated_at)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn get_skill_by_id(&self, id: Uuid) -> Result<Option<Skill>, String> {
        let row = sqlx::query(
            r#"SELECT id, canonical_name, display_name, description, format,
                      library_path, status, created_at, updated_at
               FROM skills WHERE id = ?"#
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        match row {
            Some(r) => Ok(Some(Self::row_to_skill(&r))),
            None => Ok(None),
        }
    }

    pub async fn get_skill_by_library_path(&self, path: &str) -> Result<Option<Skill>, String> {
        let row = sqlx::query(
            r#"SELECT id, canonical_name, display_name, description, format,
                      library_path, status, created_at, updated_at
               FROM skills WHERE library_path = ?"#
        )
        .bind(path)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        match row {
            Some(r) => Ok(Some(Self::row_to_skill(&r))),
            None => Ok(None),
        }
    }

    pub async fn list_skills(&self, status: Option<&str>) -> Result<Vec<Skill>, String> {
        let query = if let Some(s) = status {
            sqlx::query(
                r#"SELECT id, canonical_name, display_name, description, format,
                          library_path, status, created_at, updated_at
                   FROM skills WHERE status = ?"#
            )
            .bind(s)
        } else {
            sqlx::query(
                r#"SELECT id, canonical_name, display_name, description, format,
                          library_path, status, created_at, updated_at
                   FROM skills"#
            )
        };

        let rows = query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| e.to_string())?;

        Ok(rows.iter().map(Self::row_to_skill).collect())
    }

    pub async fn update_skill_status(&self, id: Uuid, status: SkillStatus) -> Result<(), String> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            r#"UPDATE skills SET status = ?, updated_at = ? WHERE id = ?"#
        )
        .bind(status.to_string())
        .bind(&now)
        .bind(id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── SkillVersion CRUD ─────────────────────────────────────────

    pub async fn create_skill_version(&self, version: &SkillVersion) -> Result<(), String> {
        sqlx::query(
            r#"INSERT INTO skill_versions (
                id, skill_id, version_label, source_revision,
                content_hash, manifest_json, library_snapshot_path, created_at
              ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"#
        )
        .bind(version.id.to_string())
        .bind(version.skill_id.to_string())
        .bind(&version.version_label)
        .bind(&version.source_revision)
        .bind(&version.content_hash)
        .bind(&version.manifest_json)
        .bind(&version.library_snapshot_path)
        .bind(&version.created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn get_latest_version(&self, skill_id: Uuid) -> Result<Option<SkillVersion>, String> {
        let row = sqlx::query(
            r#"SELECT id, skill_id, version_label, source_revision,
                      content_hash, manifest_json, library_snapshot_path, created_at
               FROM skill_versions
               WHERE skill_id = ?
               ORDER BY created_at DESC LIMIT 1"#
        )
        .bind(skill_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        match row {
            Some(r) => Ok(Some(Self::row_to_version(&r))),
            None => Ok(None),
        }
    }

    pub async fn list_versions(&self, skill_id: Uuid) -> Result<Vec<SkillVersion>, String> {
        let rows = sqlx::query(
            r#"SELECT id, skill_id, version_label, source_revision,
                      content_hash, manifest_json, library_snapshot_path, created_at
               FROM skill_versions
               WHERE skill_id = ?
               ORDER BY created_at DESC"#
        )
        .bind(skill_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(rows.iter().map(Self::row_to_version).collect())
    }

    // ── v0.1 import / deploy support ──────────────────────────────

    pub async fn find_by_canonical_name(&self, name: &str) -> Result<Option<Skill>, String> {
        let row = sqlx::query(
            r#"SELECT id, canonical_name, display_name, description, format,
                      library_path, status, created_at, updated_at
               FROM skills WHERE canonical_name = ?"#,
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.map(|r| Self::row_to_skill(&r)))
    }

    /// Point the skill's `current_version_id` at a freshly imported version.
    pub async fn set_current_version(
        &self,
        skill_id: Uuid,
        version_id: Uuid,
    ) -> Result<(), String> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            r#"UPDATE skills
               SET current_version_id = ?, updated_at = ?
               WHERE id = ?"#,
        )
        .bind(version_id.to_string())
        .bind(&now)
        .bind(skill_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Reuse an existing immutable version for `(skill, content_hash)` so the
    /// same content is never stored twice (test-plan invariant #5).
    pub async fn find_version_by_hash(
        &self,
        skill_id: Uuid,
        content_hash: &str,
    ) -> Result<Option<SkillVersion>, String> {
        let row = sqlx::query(
            r#"SELECT id, skill_id, version_label, source_revision,
                      content_hash, manifest_json, library_snapshot_path, created_at
               FROM skill_versions
               WHERE skill_id = ? AND content_hash = ?"#,
        )
        .bind(skill_id.to_string())
        .bind(content_hash)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.map(|r| Self::row_to_version(&r)))
    }

    pub async fn get_version(&self, version_id: Uuid) -> Result<Option<SkillVersion>, String> {
        let row = sqlx::query(
            r#"SELECT id, skill_id, version_label, source_revision,
                      content_hash, manifest_json, library_snapshot_path, created_at
               FROM skill_versions WHERE id = ?"#,
        )
        .bind(version_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(row.map(|r| Self::row_to_version(&r)))
    }

    /// Delete a skill and its versions (versions cascade via FK). Active
    /// deployments referencing it are the caller's responsibility.
    pub async fn delete_skill(&self, skill_id: Uuid) -> Result<(), String> {
        sqlx::query(r#"DELETE FROM skills WHERE id = ?"#)
            .bind(skill_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ── Helpers ───────────────────────────────────────────────────

    fn parse_uuid(s: &str) -> Uuid {
        Uuid::parse_str(s).expect("uuid string must be valid")
    }

    fn row_to_skill(row: &sqlx::sqlite::SqliteRow) -> Skill {
        let status_str: String = row.get("status");
        Skill {
            id: Self::parse_uuid(&row.get::<String, _>("id")),
            canonical_name: row.get("canonical_name"),
            display_name: row.get("display_name"),
            description: row.get("description"),
            format: row.get("format"),
            library_path: row.get("library_path"),
            status: match status_str.as_str() {
                "corrupted" => SkillStatus::Corrupted,
                "missing" => SkillStatus::Missing,
                _ => SkillStatus::Ready,
            },
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        }
    }

    fn row_to_version(row: &sqlx::sqlite::SqliteRow) -> SkillVersion {
        SkillVersion {
            id: Self::parse_uuid(&row.get::<String, _>("id")),
            skill_id: Self::parse_uuid(&row.get::<String, _>("skill_id")),
            version_label: row.get("version_label"),
            source_revision: row.get("source_revision"),
            content_hash: row.get("content_hash"),
            manifest_json: row.get("manifest_json"),
            library_snapshot_path: row.get("library_snapshot_path"),
            created_at: row.get("created_at"),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────

