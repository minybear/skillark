//! Audit-log repository for write operations (import / install / uninstall / verify).

use std::str::FromStr;

use sqlx::{Row, SqlitePool};

use crate::domain::operation::{Operation, OperationStatus, OperationType};

pub struct OperationRepository {
    pool: SqlitePool,
}

impl OperationRepository {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Create a row with status `running`. Returns to its terminal state via [`complete`].
    pub async fn create(
        &self,
        id: &str,
        operation_type: OperationType,
        plan_json: &str,
    ) -> Result<(), String> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            r#"INSERT INTO operations (id, operation_type, status, plan_json, result_json,
                                       error_code, error_message, started_at, completed_at)
               VALUES (?, ?, 'running', ?, NULL, NULL, NULL, ?, NULL)"#,
        )
        .bind(id)
        .bind(operation_type.as_str())
        .bind(plan_json)
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Advance an operation to a terminal status, recording its result and error.
    pub async fn complete(
        &self,
        id: &str,
        status: OperationStatus,
        result_json: Option<&str>,
        error_message: Option<&str>,
    ) -> Result<(), String> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            r#"UPDATE operations
               SET status = ?, result_json = ?, error_message = ?, completed_at = ?
               WHERE id = ?"#,
        )
        .bind(status.as_str())
        .bind(result_json)
        .bind(error_message)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn get(&self, id: &str) -> Result<Option<Operation>, String> {
        let row = sqlx::query(
            r#"SELECT id, operation_type, status, plan_json, result_json,
                      error_code, error_message, started_at, completed_at
               FROM operations WHERE id = ?"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(row.map(|r| Self::row_to_operation(&r)))
    }

    pub async fn list_recent(&self, limit: i64) -> Result<Vec<Operation>, String> {
        let rows = sqlx::query(
            r#"SELECT id, operation_type, status, plan_json, result_json,
                      error_code, error_message, started_at, completed_at
               FROM operations ORDER BY started_at DESC LIMIT ?"#,
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| e.to_string())?;

        Ok(rows.iter().map(Self::row_to_operation).collect())
    }

    /// Convert operations left `running` by a prior process crash into explicit
    /// failed audit records during the next startup.
    pub async fn recover_interrupted(&self) -> Result<u64, String> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            r#"UPDATE operations
               SET status = 'failed',
                   error_code = 'interrupted',
                   error_message = 'The previous SkillArk process exited before this operation completed.',
                   completed_at = ?
               WHERE status = 'running'"#,
        )
        .bind(&now)
        .execute(&self.pool)
        .await
        .map_err(|error| error.to_string())?;
        Ok(result.rows_affected())
    }

    fn row_to_operation(row: &sqlx::sqlite::SqliteRow) -> Operation {
        Operation {
            id: row.get("id"),
            operation_type: OperationType::from_str(row.get::<&str, _>("operation_type"))
                .unwrap_or(OperationType::Install),
            status: OperationStatus::from_str(row.get::<&str, _>("status"))
                .unwrap_or(OperationStatus::Failed),
            plan_json: row.get("plan_json"),
            result_json: row.get("result_json"),
            error_code: row.get("error_code"),
            error_message: row.get("error_message"),
            started_at: row.get("started_at"),
            completed_at: row.get("completed_at"),
        }
    }
}
