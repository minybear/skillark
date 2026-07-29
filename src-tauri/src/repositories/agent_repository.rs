//! Agent persistence repository backed by SQLite.
//!
//! Uses raw `sqlx::query` + manual Row→tuple mapping for in-memory test compatibility.

use sqlx::{SqlitePool, Row};
use uuid::Uuid;

pub struct AgentRepository {
    pool: SqlitePool,
}

impl AgentRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn get_id_by_type(
        &self,
        agent_type: &str,
    ) -> Result<Option<Uuid>, String> {
        let row = sqlx::query(
            r#"SELECT id FROM agents WHERE agent_type = ?"#,
        )
        .bind(agent_type)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        let Some(r) = row else { return Ok(None); };
        let id_str: String = r.get("id");
        Ok(Uuid::parse_str(&id_str).ok())
    }

    /// Insert or update an agent row keyed on `agent_type` (upsert).
    /// Returns the agent's UUID.
    pub async fn upsert_agent(
        &self,
        agent_type: &str,
        display_name: &str,
        executable_path: Option<String>,
        global_skill_path: Option<String>,
        confidence: i32,
        user_configured: bool,
    ) -> Result<Uuid, String> {
        let id = Uuid::new_v4();

        // Try insert first; if agent_type already exists, update it instead.
        let result = sqlx::query(
            r#"INSERT INTO agents (
                id, agent_type, display_name, environment, executable_path,
                global_skill_path, status, confidence, user_configured, enabled,
                config_json, created_at, updated_at
              ) VALUES (?, ?, ?, 'windows', ?, ?, 'detected', ?, ?, 1, '{}', ?, ?)"#
        )
        .bind(id.to_string())
        .bind(agent_type)
        .bind(display_name)
        .bind(&executable_path)
        .bind(&global_skill_path)
        .bind(confidence)
        .bind(user_configured as i64)
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => Ok(id),
            Err(e) if e.to_string().contains("UNIQUE") || e.to_string().contains("constraint") => {
                // Agent with this type exists — update it.
                sqlx::query(
                    r#"UPDATE agents SET
                        display_name = ?,
                        executable_path = ?,
                        global_skill_path = ?,
                        confidence = ?,
                        user_configured = ?,
                        updated_at = ?
                    WHERE agent_type = ?"#,
                )
                .bind(display_name)
                .bind(executable_path)
                .bind(global_skill_path)
                .bind(confidence)
                .bind(user_configured as i64)
                .bind(chrono::Utc::now().to_rfc3339())
                .bind(agent_type)
                .execute(&self.pool)
                .await
                .map_err(|e| e.to_string())?;

                // Fetch the existing id.
                let row = sqlx::query(
                    r#"SELECT id FROM agents WHERE agent_type = ?"#,
                )
                .bind(agent_type)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| e.to_string())?;

                let id_str: String = row.get("id");
                Ok(Uuid::parse_str(&id_str).map_err(|e| e.to_string())?)
            }
            Err(e) => Err(e.to_string()),
        }
    }

    /// Get a single agent by its type string.
    pub async fn get_agent_by_type(
        &self,
        agent_type: &str,
    ) -> Result<Option<(Uuid, String, String, Option<String>, Option<String>, i32, bool)>, String>
    {
        let row = sqlx::query(
            r#"SELECT id, agent_type, display_name, executable_path,
                      global_skill_path, confidence, user_configured
               FROM agents WHERE agent_type = ?"#,
        )
        .bind(agent_type)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        match row {
            Some(r) => Ok(Some(Self::row_to_agent(&r))),
            None => Ok(None),
        }
    }

    /// List all agents.
    pub async fn list_agents(&self) -> Result<Vec<(Uuid, String, String, Option<String>, Option<String>, i32, bool)>, String>
    {
        let rows = sqlx::query(
            r#"SELECT id, agent_type, display_name, executable_path,
                      global_skill_path, confidence, user_configured
               FROM agents"#,
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(rows.iter().map(Self::row_to_agent).collect())
    }

    fn row_to_agent(row: &sqlx::sqlite::SqliteRow) -> (Uuid, String, String, Option<String>, Option<String>, i32, bool) {
        let id_str: String = row.get("id");
        (
            Uuid::parse_str(&id_str).expect("valid uuid"),
            row.get("agent_type"),
            row.get("display_name"),
            row.get("executable_path"),
            row.get("global_skill_path"),
            row.get("confidence"),
            row.get::<i64, _>("user_configured") != 0,
        )
    }
}

// ── Tests ─────────────────────────────────────────────────────────

