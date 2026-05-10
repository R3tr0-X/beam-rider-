use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

use crate::domain::{MarketSignal, SignalKind, normalize_pair};
use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct StoredSignal {
    pub id: i64,
    pub signal: MarketSignal,
    pub signature: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct SqliteSignalRepository {
    pool: SqlitePool,
}

impl SqliteSignalRepository {
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn insert_signed(
        &self,
        signal: &MarketSignal,
        signature: &[u8],
    ) -> Result<i64, AppError> {
        let result = sqlx::query(
            "INSERT INTO signals (pair, kind, value_bps, confidence, created_at, signature)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&signal.pair)
        .bind(signal.kind.as_i64())
        .bind(signal.value_bps)
        .bind(signal.confidence.value())
        .bind(signal.created_at.to_rfc3339())
        .bind(signature)
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn last_n_for_pair(
        &self,
        pair: &str,
        limit: i64,
    ) -> Result<Vec<StoredSignal>, AppError> {
        let pair = normalize_pair(pair).map_err(AppError::BadRequest)?;
        let rows = sqlx::query(
            "SELECT id, pair, kind, value_bps, confidence, created_at, signature
             FROM signals
             WHERE pair = ?
             ORDER BY created_at DESC, id DESC
             LIMIT ?",
        )
        .bind(pair)
        .bind(limit.max(0))
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_signal).collect()
    }

    pub async fn latest(&self) -> Result<Option<StoredSignal>, AppError> {
        let row = sqlx::query(
            "SELECT id, pair, kind, value_bps, confidence, created_at, signature
             FROM signals
             ORDER BY created_at DESC, id DESC
             LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;

        row.map(row_to_signal).transpose()
    }

    pub async fn count(&self) -> Result<i64, AppError> {
        let row = sqlx::query("SELECT COUNT(*) AS count FROM signals")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.try_get("count")?)
    }
}

pub(crate) fn parse_utc(value: &str) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|err| AppError::BadRequest(format!("invalid stored timestamp: {err}")))
}

pub(crate) fn row_to_signal(row: sqlx::sqlite::SqliteRow) -> Result<StoredSignal, AppError> {
    let kind_raw: i64 = row.try_get("kind")?;
    let kind = SignalKind::from_i64(kind_raw)
        .ok_or_else(|| AppError::BadRequest(format!("invalid stored signal kind: {kind_raw}")))?;
    let confidence: f64 = row.try_get("confidence")?;

    Ok(StoredSignal {
        id: row.try_get("id")?,
        signal: MarketSignal::new(
            row.try_get::<String, _>("pair")?,
            kind,
            row.try_get("value_bps")?,
            confidence,
            parse_utc(&row.try_get::<String, _>("created_at")?)?,
        )
        .map_err(AppError::BadRequest)?,
        signature: row.try_get("signature")?,
    })
}
