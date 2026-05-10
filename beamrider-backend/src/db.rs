use std::time::Duration;

use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};

use crate::error::AppError;

pub async fn connect(database_url: &str, max_connections: u32) -> Result<SqlitePool, AppError> {
    let mut options = database_url
        .parse::<SqliteConnectOptions>()
        .map_err(|err| AppError::Config(format!("invalid DATABASE_URL: {err}")))?
        .create_if_missing(true)
        .foreign_keys(true)
        .busy_timeout(Duration::from_secs(5));

    if !database_url.contains(":memory:") {
        options = options
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal);
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(max_connections)
        .acquire_timeout(Duration::from_secs(5))
        .connect_with(options)
        .await?;

    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}

pub async fn connect_memory() -> Result<SqlitePool, AppError> {
    connect("sqlite::memory:", 1).await
}
