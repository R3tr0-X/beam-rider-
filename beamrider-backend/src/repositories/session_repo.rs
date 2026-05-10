use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct StoredSession {
    pub token: String,
    pub buyer: String,
    pub chain_id: i64,
    pub paid_token: String,
    pub paid_amount: String,
    pub balance: i64,
    pub requests_used: i64,
    pub expiry: DateTime<Utc>,
    pub settle_tx_hash: String,
}

#[derive(Debug, Clone)]
pub struct SqliteSessionRepository {
    pool: SqlitePool,
}

impl SqliteSessionRepository {
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn insert(&self, session: &StoredSession) -> Result<(), AppError> {
        let result = sqlx::query(
            "INSERT INTO sessions
             (token, buyer, chain_id, paid_token, paid_amount, balance, requests_used,
              expiry, settle_tx_hash)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&session.token)
        .bind(&session.buyer)
        .bind(session.chain_id)
        .bind(&session.paid_token)
        .bind(&session.paid_amount)
        .bind(session.balance)
        .bind(session.requests_used)
        .bind(session.expiry.to_rfc3339())
        .bind(&session.settle_tx_hash)
        .execute(&self.pool)
        .await;

        match result {
            Ok(_) => Ok(()),
            Err(err) if is_unique(&err) => Err(AppError::Conflict(
                "session token or settle_tx_hash already exists".to_string(),
            )),
            Err(err) => Err(err.into()),
        }
    }

    /// Atomically consume one credit from a live session. Returns the
    /// updated session if the decrement succeeded, otherwise an error.
    pub async fn consume_one(&self, token: &str) -> Result<StoredSession, AppError> {
        let now = Utc::now().to_rfc3339();
        let updated = sqlx::query(
            "UPDATE sessions
             SET balance = balance - 1,
                 requests_used = requests_used + 1
             WHERE token = ? AND balance > 0 AND expiry > ?",
        )
        .bind(token)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        if updated.rows_affected() == 0 {
            return Err(AppError::PaymentVerification(
                "session is exhausted, expired, or unknown".to_string(),
            ));
        }

        self.get(token).await?.ok_or_else(|| {
            AppError::PaymentVerification("session vanished after decrement".to_string())
        })
    }

    pub async fn get(&self, token: &str) -> Result<Option<StoredSession>, AppError> {
        let row = sqlx::query(
            "SELECT token, buyer, chain_id, paid_token, paid_amount, balance,
                    requests_used, expiry, settle_tx_hash
             FROM sessions WHERE token = ?",
        )
        .bind(token)
        .fetch_optional(&self.pool)
        .await?;
        row.map(row_to_session).transpose()
    }

    pub async fn count_active(&self) -> Result<i64, AppError> {
        let row = sqlx::query(
            "SELECT COUNT(*) AS count FROM sessions
             WHERE balance > 0 AND expiry > ?",
        )
        .bind(Utc::now().to_rfc3339())
        .fetch_one(&self.pool)
        .await?;
        Ok(row.try_get("count")?)
    }
}

fn row_to_session(row: sqlx::sqlite::SqliteRow) -> Result<StoredSession, AppError> {
    let expiry_raw: String = row.try_get("expiry")?;
    let expiry = DateTime::parse_from_rfc3339(&expiry_raw)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|err| AppError::BadRequest(format!("invalid stored expiry: {err}")))?;
    Ok(StoredSession {
        token: row.try_get("token")?,
        buyer: row.try_get("buyer")?,
        chain_id: row.try_get("chain_id")?,
        paid_token: row.try_get("paid_token")?,
        paid_amount: row.try_get("paid_amount")?,
        balance: row.try_get("balance")?,
        requests_used: row.try_get("requests_used")?,
        expiry,
        settle_tx_hash: row.try_get("settle_tx_hash")?,
    })
}

fn is_unique(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(db) => db.message().to_ascii_lowercase().contains("unique"),
        _ => false,
    }
}
