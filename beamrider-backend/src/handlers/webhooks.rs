use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};

use crate::domain::RebalanceStatus;
use crate::error::AppError;
use crate::state::AppState;

#[derive(Debug, Clone, Deserialize)]
pub struct CctpWebhookRequest {
    pub rebalance_id: i64,
    pub status: RebalanceStatus,
    #[serde(default)]
    pub tx_hash: Option<String>,
    #[serde(default)]
    pub attestation_hex: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CctpWebhookResponse {
    pub accepted: bool,
}

pub async fn cctp_webhook(
    State(state): State<AppState>,
    Json(request): Json<CctpWebhookRequest>,
) -> Result<Json<CctpWebhookResponse>, AppError> {
    let attestation = request
        .attestation_hex
        .as_deref()
        .map(|value| {
            let hex_value = value.strip_prefix("0x").unwrap_or(value);
            hex::decode(hex_value).map_err(|err| AppError::BadRequest(err.to_string()))
        })
        .transpose()?;

    state
        .event_repo
        .update_cctp_status(
            request.rebalance_id,
            request.status,
            request.tx_hash.as_deref(),
            attestation.as_deref(),
        )
        .await?;

    Ok(Json(CctpWebhookResponse { accepted: true }))
}
