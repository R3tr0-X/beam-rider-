use chrono::{DateTime, Utc};
use sqlx::{Row, SqlitePool};

use crate::domain::{RebalancePlan, RebalanceStatus, Venue};
use crate::error::AppError;
use crate::repositories::signal_repo::parse_utc;

#[derive(Debug, Clone)]
pub struct StoredRebalance {
    pub id: i64,
    pub plan: RebalancePlan,
    pub propose_tx: Option<String>,
    pub bridge_tx: Option<String>,
    pub cctp_burn_tx: Option<String>,
    pub cctp_mint_tx: Option<String>,
    pub finished_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct SqliteEventRepository {
    pool: SqlitePool,
}

impl SqliteEventRepository {
    pub const fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn insert_rebalance(&self, plan: &RebalancePlan) -> Result<i64, AppError> {
        let result = sqlx::query(
            "INSERT INTO rebalances
             (proposed_at, src_chain, dest_chain, amount_usdc_atoms, venue, expected_apy_bps, status)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(plan.proposed_at.to_rfc3339())
        .bind(plan.src_chain)
        .bind(plan.dest_chain)
        .bind(&plan.amount_usdc_atoms)
        .bind(plan.venue.as_str())
        .bind(plan.expected_apy_bps)
        .bind(plan.status.as_str())
        .execute(&self.pool)
        .await?;

        Ok(result.last_insert_rowid())
    }

    pub async fn latest_rebalance(&self) -> Result<Option<StoredRebalance>, AppError> {
        let row = sqlx::query(
            "SELECT id, proposed_at, src_chain, dest_chain, amount_usdc_atoms, venue,
                    expected_apy_bps, propose_tx, bridge_tx, cctp_burn_tx, cctp_mint_tx,
                    status, finished_at
             FROM rebalances
             ORDER BY proposed_at DESC, id DESC
             LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;

        row.map(row_to_rebalance).transpose()
    }

    pub async fn count_rebalances(&self) -> Result<i64, AppError> {
        let row = sqlx::query("SELECT COUNT(*) AS count FROM rebalances")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.try_get("count")?)
    }

    pub async fn update_cctp_status(
        &self,
        id: i64,
        status: RebalanceStatus,
        tx_hash: Option<&str>,
        attestation: Option<&[u8]>,
    ) -> Result<(), AppError> {
        let finished_at = matches!(status, RebalanceStatus::Completed | RebalanceStatus::Failed)
            .then(|| Utc::now().to_rfc3339());

        let result = sqlx::query(
            "UPDATE rebalances
             SET status = ?,
                 cctp_burn_tx = CASE WHEN ? = 'cctp_burnt' THEN COALESCE(?, cctp_burn_tx) ELSE cctp_burn_tx END,
                 cctp_mint_tx = CASE WHEN ? = 'completed' THEN COALESCE(?, cctp_mint_tx) ELSE cctp_mint_tx END,
                 cctp_attestation = COALESCE(?, cctp_attestation),
                 finished_at = COALESCE(?, finished_at)
             WHERE id = ?",
        )
        .bind(status.as_str())
        .bind(status.as_str())
        .bind(tx_hash)
        .bind(status.as_str())
        .bind(tx_hash)
        .bind(attestation)
        .bind(finished_at)
        .bind(id)
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(format!("rebalance {id}")));
        }
        Ok(())
    }
}

fn row_to_rebalance(row: sqlx::sqlite::SqliteRow) -> Result<StoredRebalance, AppError> {
    let venue_raw: String = row.try_get("venue")?;
    let venue = Venue::parse_db(&venue_raw)
        .ok_or_else(|| AppError::BadRequest(format!("invalid stored venue: {venue_raw}")))?;
    let status_raw: String = row.try_get("status")?;
    let status = RebalanceStatus::parse_db(&status_raw)
        .ok_or_else(|| AppError::BadRequest(format!("invalid stored status: {status_raw}")))?;
    let finished_raw: Option<String> = row.try_get("finished_at")?;

    Ok(StoredRebalance {
        id: row.try_get("id")?,
        plan: RebalancePlan {
            proposed_at: parse_utc(&row.try_get::<String, _>("proposed_at")?)?,
            src_chain: row.try_get("src_chain")?,
            dest_chain: row.try_get("dest_chain")?,
            amount_usdc_atoms: row.try_get("amount_usdc_atoms")?,
            venue,
            expected_apy_bps: row.try_get("expected_apy_bps")?,
            status,
        },
        propose_tx: row.try_get("propose_tx")?,
        bridge_tx: row.try_get("bridge_tx")?,
        cctp_burn_tx: row.try_get("cctp_burn_tx")?,
        cctp_mint_tx: row.try_get("cctp_mint_tx")?,
        finished_at: finished_raw.as_deref().map(parse_utc).transpose()?,
    })
}
