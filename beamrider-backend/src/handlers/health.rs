use axum::{Json, extract::State};
use chrono::Utc;

use crate::dto::HealthResponse;
use crate::error::AppError;
use crate::state::AppState;

pub async fn healthz(State(state): State<AppState>) -> Result<Json<HealthResponse>, AppError> {
    sqlx::query("SELECT 1").execute(&state.pool).await?;
    Ok(Json(HealthResponse {
        service: "beamrider-backend".to_string(),
        database: "ok".to_string(),
        timestamp: Utc::now(),
    }))
}
