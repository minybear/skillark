//! Database backup helper (Task 13 release-prep slice).
//!
//! Produces a self-contained, recoverable copy of the SQLite database via
//! `VACUUM INTO`, which snapshots the live DB (including WAL state) into a new
//! file. Timestamped so repeated backups don't overwrite each other.

use std::path::{Path, PathBuf};

use sqlx::Connection;

#[derive(Debug, thiserror::Error)]
pub enum BackupError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("sqlite backup failed: {0}")]
    Sqlite(String),
}

/// Snapshot the database at `source_url` (a `sqlite:` URL) into `dest_dir`.
pub async fn backup_database_file(source_url: &str, dest_dir: &Path) -> Result<PathBuf, BackupError> {
    std::fs::create_dir_all(dest_dir)?;

    let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%S");
    let dest = dest_dir.join(format!("skillark-{stamp}.db"));

    let mut conn = sqlx::sqlite::SqliteConnection::connect(source_url)
        .await
        .map_err(|e| BackupError::Sqlite(e.to_string()))?;
    // VACUUM INTO writes a fresh, compacted, consistent copy to a plain file
    // path (not a sqlite: URL). It cannot run inside a transaction; sqlx runs
    // it autocommit on a plain connection.
    sqlx::query(&format!(
        "VACUUM INTO '{}';",
        dest.to_string_lossy().replace('\'', "''")
    ))
    .execute(&mut conn)
    .await
    .map_err(|e| BackupError::Sqlite(e.to_string()))?;
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::sqlite::connect;

    fn unique_tmp(sub: &str) -> PathBuf {
        let home = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let mut p = home.join(".skillark-backup-test");
        p.push(format!(
            "{}-{}-{sub}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[tokio::test]
    async fn backup_is_a_queryable_copy() {
        let dir = unique_tmp("ok");
        let src_path = dir.join("src.sqlite");
        let src_url = format!("sqlite:{}", src_path.to_string_lossy());

        // Create + populate the source through the real migration path.
        let pool = connect(&src_url).await.unwrap();
        sqlx::query("INSERT INTO workspaces (id, workspace_type, name, root_path, status, created_at, updated_at) VALUES ('w', 'global', 'w', NULL, 'available', 't', 't')")
            .execute(&pool)
            .await
            .unwrap();
        drop(pool);

        let dest_dir = dir.join("backups");
        let dest = backup_database_file(&src_url, &dest_dir).await.unwrap();
        assert!(dest.is_file());

        // The backup must be openable and contain the row we wrote.
        let backup_url = format!("sqlite:{}", dest.to_string_lossy());
        let pool2 = connect(&backup_url).await.unwrap();
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM workspaces WHERE id = 'w'")
            .fetch_one(&pool2)
            .await
            .unwrap();
        assert_eq!(count, 1, "backup must preserve written rows");
    }
}
