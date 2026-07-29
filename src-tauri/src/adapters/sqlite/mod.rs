use std::str::FromStr;

use sqlx::{
    migrate::MigrateError,
    sqlite::{SqliteConnectOptions, SqlitePoolOptions},
    SqlitePool,
};

#[derive(Debug, thiserror::Error)]
pub enum SqliteStoreError {
    #[error("SQLite connection failed: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("SQLite migration failed: {0}")]
    Migration(#[from] MigrateError),
}

pub async fn connect(database_url: &str) -> Result<SqlitePool, SqliteStoreError> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .create_if_missing(true)
        .foreign_keys(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}
