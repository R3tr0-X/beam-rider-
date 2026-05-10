use sqlx::{Row, SqlitePool};

use crate::domain::{MarketSignal, VerifiedStacksSale};
use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct SqliteStacksSaleRepository {
    pool: SqlitePool,
}

impl SqliteStacksSaleRepository {
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Insert a (signal, stacks_sales) pair atomically. Mirrors the EVM
    /// `SqliteSaleRepository::insert_signal_and_sale` shape so the service
    /// layer reads uniformly across payment surfaces.
    pub async fn insert_signal_and_sale(
        &self,
        signal: &MarketSignal,
        signature: &[u8],
        sale: &VerifiedStacksSale,
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
            "INSERT INTO stacks_sales
             (signal_id, buyer, pair, token, amount_atoms, stacks_tx_id, block_height, settled_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(signal_id)
        .bind(&sale.buyer)
        .bind(&sale.pair)
        .bind(sale.token.as_str())
        .bind(&sale.amount_atoms)
        .bind(&sale.stacks_tx_id)
        .bind(sale.block_height)
        .bind(sale.settled_at.to_rfc3339())
        .execute(&mut *tx)
        .await;

        match sale_result {
            Ok(result) => {
                tx.commit().await?;
                Ok((signal_id, result.last_insert_rowid()))
            }
            Err(err) if is_unique(&err) => {
                tx.rollback().await?;
                Err(AppError::Conflict(
                    "stacks sale tx already recorded".to_string(),
                ))
            }
            Err(err) => {
                tx.rollback().await?;
                Err(err.into())
            }
        }
    }

    pub async fn count(&self) -> Result<i64, AppError> {
        let row = sqlx::query("SELECT COUNT(*) AS count FROM stacks_sales")
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
