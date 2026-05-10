use axum::{Json, extract::State, http::HeaderMap};

use crate::dto::{IssueSessionRequest, IssueSessionResponse};
use crate::error::AppError;
use crate::state::AppState;

pub async fn issue_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Option<Json<IssueSessionRequest>>,
) -> Result<Json<IssueSessionResponse>, AppError> {
    let payment = state.x402.verify_headers(&headers, "session").await?;
    let requests = body.and_then(|Json(req)| req.requests);
    let issued = state.session_service.issue(&payment, requests).await?;
    Ok(Json(IssueSessionResponse {
        token: issued.token,
        balance: issued.balance,
        expiry: issued.expiry,
    }))
}
