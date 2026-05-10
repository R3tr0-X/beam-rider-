use sqlx::{Row, SqlitePool};

use crate::domain::{MarketSignal, VerifiedPayment};
use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct SqliteMiniPayRepository {
    pool: SqlitePool,
}

impl SqliteMiniPayRepository {
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn insert_signal_and_payment(
        &self,
        signal: &MarketSignal,
        signature: &[u8],
        payment: &VerifiedPayment,
        block_number: i64,
    ) -> Result<(i64, i64), AppError> {
        let mut tx = self.pool.begin().await?;

        let signal_result = sqlx::query(
            "INSERT INTO signals (pair, kind, value_bps, confidence, created_at, signature)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&signal.pair)
        .bind(signal.kind.as_i64())
        .bind(signal.value_bps)
        .bind(signal.confidence.value())
        .bind(signal.created_at.to_rfc3339())
        .bind(signature)
        .execute(&mut *tx)
        .await?;
        let signal_id = signal_result.last_insert_rowid();

        let payment_result = sqlx::query(
            "INSERT INTO minipay_payments
             (signal_id, buyer, pair, amount_atoms, token, tx_hash, block_number, settled_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(signal_id)
        .bind(&payment.buyer)
        .bind(&signal.pair)
        .bind(&payment.amount_atoms)
        .bind(&payment.token)
        .bind(&payment.tx_hash)
        .bind(block_number)
        .bind(payment.settled_at.to_rfc3339())
        .execute(&mut *tx)
        .await;

        match payment_result {
            Ok(result) => {
                tx.commit().await?;
                Ok((signal_id, result.last_insert_rowid()))
            }
            Err(err) if is_unique(&err) => {
                tx.rollback().await?;
                Err(AppError::Conflict(
                    "minipay tx already recorded".to_string(),
                ))
            }
            Err(err) => {
                tx.rollback().await?;
                Err(err.into())
            }
        }
    }

    pub async fn count(&self) -> Result<i64, AppError> {
        let row = sqlx::query("SELECT COUNT(*) AS count FROM minipay_payments")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.try_get("count")?)
    }
}

fn is_unique(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(db) => db.message().to_ascii_lowercase().contains("unique"),
        _ => false,
    }
}
