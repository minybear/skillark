//! Persisted "disabled agent" set (PRD §5.2).
//!
//! Stored in the `app_settings` table under `disabled_agents` so the flag
//! survives independently of live discovery (discovered agents are not otherwise
//! persisted). Disabled agents are excluded from deploy target lists.

use sqlx::Row;
use sqlx::SqlitePool;

const KEY: &str = "disabled_agents";

pub async fn load_disabled(pool: &SqlitePool) -> Result<Vec<String>, String> {
    let row = sqlx::query(r#"SELECT value_json FROM app_settings WHERE key = ?"#)
        .bind(KEY)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?;

    let Some(row) = row else {
        return Ok(Vec::new());
    };
    let value: String = row.get("value_json");
    Ok(serde_json::from_str::<Vec<String>>(&value).unwrap_or_default())
}

pub async fn set_disabled(
    pool: &SqlitePool,
    agent_type: &str,
    disabled: bool,
) -> Result<Vec<String>, String> {
    let mut current = load_disabled(pool).await?;
    if disabled {
        if !current.iter().any(|a| a == agent_type) {
            current.push(agent_type.to_owned());
        }
    } else {
        current.retain(|a| a != agent_type);
    }
    write_disabled(pool, &current).await?;
    Ok(current)
}

async fn write_disabled(pool: &SqlitePool, list: &[String]) -> Result<(), String> {
    let now = chrono::Utc::now().to_rfc3339();
    let json = serde_json::to_string(list).map_err(|e| e.to_string())?;
    sqlx::query(
        r#"INSERT INTO app_settings (key, value_json, updated_at)
           VALUES (?, ?, ?)
           ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json, updated_at = excluded.updated_at"#,
    )
    .bind(KEY)
    .bind(&json)
    .bind(&now)
    .execute(pool)
    .await
    .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup() -> SqlitePool {
        let home = std::env::var_os("USERPROFILE")
            .or_else(|| std::env::var_os("HOME"))
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let mut p = home.join(".skillark-disabled-test");
        std::fs::create_dir_all(&p).unwrap();
        p.push(format!(
            "{}-{}.sqlite",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let url = format!("sqlite:{}", p.to_string_lossy());
        crate::adapters::sqlite::connect(&url).await.unwrap()
    }

    #[tokio::test]
    async fn toggle_disabled_round_trip() {
        let pool = setup().await;
        assert!(load_disabled(&pool).await.unwrap().is_empty());

        let after_on = set_disabled(&pool, "codex", true).await.unwrap();
        assert!(after_on.iter().any(|a| a == "codex"));
        assert!(load_disabled(&pool).await.unwrap().iter().any(|a| a == "codex"));

        let _ = set_disabled(&pool, "codex", true).await.unwrap(); // idempotent
        assert_eq!(load_disabled(&pool).await.unwrap().iter().filter(|a| *a == "codex").count(), 1);

        let after_off = set_disabled(&pool, "codex", false).await.unwrap();
        assert!(after_off.iter().all(|a| a != "codex"));
    }
}
