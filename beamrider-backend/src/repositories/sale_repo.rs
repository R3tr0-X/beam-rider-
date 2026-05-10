use sqlx::{Row, SqlitePool};

use crate::domain::{MarketSignal, VerifiedPayment};
use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct SqliteSaleRepository {
    pool: SqlitePool,
}

impl SqliteSaleRepository {
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn insert_signal_and_sale(
        &self,
        signal: &MarketSignal,
        signature: &[u8],
        payment: &VerifiedPayment,
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

        let sale_result = sqlx::query(
            "INSERT INTO sales (signal_id, buyer, chain_id, token, amount_atoms, tx_hash, settled_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(signal_id)
        .bind(&payment.buyer)
        .bind(payment.chain_id)
        .bind(&payment.token)
        .bind(&payment.amount_atoms)
        .bind(&payment.tx_hash)
        .bind(payment.settled_at.to_rfc3339())
        .execute(&mut *tx)
        .await;

        match sale_result {
            Ok(result) => {
                tx.commit().await?;
                Ok((signal_id, result.last_insert_rowid()))
            }
            Err(err) if is_unique_error(&err) => {
                tx.rollback().await?;
                Err(AppError::Conflict(
                    "sale tx_hash already exists".to_string(),
                ))
            }
            Err(err) => {
                tx.rollback().await?;
                Err(err.into())
            }
        }
    }

    pub async fn count(&self) -> Result<i64, AppError> {
        let row = sqlx::query("SELECT COUNT(*) AS count FROM sales")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.try_get("count")?)
    }
}

pub(crate) fn is_unique_error(err: &sqlx::Error) -> bool {
    match err {
        sqlx::Error::Database(db) => db.message().to_ascii_lowercase().contains("unique"),
        _ => false,
    }
}
