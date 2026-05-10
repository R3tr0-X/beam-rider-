use axum::{
    Router,
    routing::{get, post},
};
use tower_http::{cors::CorsLayer, limit::RequestBodyLimitLayer, trace::TraceLayer};

use crate::handlers::{compute, health, sessions, signals, status, webhooks};
use crate::state::AppState;

pub fn router(state: AppState) -> Router {
    let request_limit = state.config.request_body_limit_bytes;

    Router::new()
        .route("/healthz", get(health::healthz))
        .route("/v1/signals/{pair}", get(signals::get_signal))
        .route("/v1/sessions", post(sessions::issue_session))
        .route("/v1/compute", post(compute::compute))
        .route("/v1/agent/status", get(status::agent_status))
        .route("/v1/webhooks/cctp", post(webhooks::cctp_webhook))
        .layer(RequestBodyLimitLayer::new(request_limit))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}
