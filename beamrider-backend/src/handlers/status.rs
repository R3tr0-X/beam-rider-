use axum::{Json, extract::State};
use chrono::Utc;

use crate::dto::{AgentStatusResponse, RebalanceSummary, StoredSignalResponse};
use crate::error::AppError;
use crate::state::AppState;

pub async fn agent_status(
    State(state): State<AppState>,
) -> Result<Json<AgentStatusResponse>, AppError> {
    let signal_count = state.signal_repo.count().await?;
    let latest_signal =
        state.signal_repo.latest().await?.map(|signal| {
            StoredSignalResponse::from_stored(signal, &state.signer.public_key_bytes())
        });
    let rebalance_count = state.event_repo.count_rebalances().await?;
    let latest_rebalance = state
        .event_repo
        .latest_rebalance()
        .await?
        .map(RebalanceSummary::from);
    let stacks_sale_count = state.stacks_sale_repo.count().await?;
    let minipay_payment_count = state.minipay_repo.count().await?;
    let active_session_count = state.session_service.count_active().await?;

    Ok(Json(AgentStatusResponse {
        service: "beamrider-backend".to_string(),
        signal_count,
        latest_signal,
        rebalance_count,
        latest_rebalance,
        stacks_sale_count,
        minipay_payment_count,
        active_session_count,
        timestamp: Utc::now(),
    }))
}
