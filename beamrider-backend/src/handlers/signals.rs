use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};

use crate::domain::{StacksToken, normalize_pair};
use crate::dto::SignalResponse;
use crate::error::AppError;
use crate::services::SignalPayment;
use crate::state::AppState;

const HEADER_SESSION: &str = "x402-session";
const HEADER_MINIPAY_TX: &str = "x-minipay-tx-hash";
const HEADER_STACKS_TX: &str = "x-stacks-tx-id";
const HEADER_STACKS_BUYER: &str = "x-stacks-buyer";
const HEADER_STACKS_TOKEN: &str = "x-stacks-token";

pub async fn get_signal(
    State(state): State<AppState>,
    Path(pair): Path<String>,
    headers: HeaderMap,
) -> Result<Json<SignalResponse>, AppError> {
    let pair = normalize_pair(&pair).map_err(AppError::BadRequest)?;
    let payment = resolve_payment(&state, &headers, &pair).await?;
    let signed = state.signal_service.produce(&pair, payment).await?;
    Ok(Json(SignalResponse::from(signed)))
}

async fn resolve_payment(
    state: &AppState,
    headers: &HeaderMap,
    pair: &str,
) -> Result<SignalPayment, AppError> {
    if let Some(token) = trimmed_header(headers, HEADER_SESSION) {
        let payment = state.session_service.consume(token).await?;
        return Ok(SignalPayment::X402(payment));
    }

    if let Some(tx_hash) = trimmed_header(headers, HEADER_MINIPAY_TX) {
        if !state.celo_payment.is_enabled() {
            return Err(AppError::PaymentVerification(
                "MiniPay verifier is not enabled".to_string(),
            ));
        }
        let payment = state.celo_payment.verify(tx_hash).await?;
        let block_number = parse_block_number(headers)?;
        return Ok(SignalPayment::MiniPay {
            payment,
            block_number,
        });
    }

    if let Some(tx_id) = trimmed_header(headers, HEADER_STACKS_TX) {
        if !state.stacks_payment.is_enabled() {
            return Err(AppError::PaymentVerification(
                "Stacks payment verifier is not enabled".to_string(),
            ));
        }
        let buyer = trimmed_header(headers, HEADER_STACKS_BUYER)
            .ok_or_else(|| AppError::BadRequest(format!("missing {HEADER_STACKS_BUYER} header")))?;
        let token = parse_stacks_token(headers)?;
        let sale = state
            .stacks_payment
            .verify(tx_id, buyer, pair, token, 1)
            .await?;
        return Ok(SignalPayment::Stacks(sale));
    }

    let payment = state.x402.verify_headers(headers, pair).await?;
    Ok(SignalPayment::X402(payment))
}

fn trimmed_header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn parse_block_number(headers: &HeaderMap) -> Result<i64, AppError> {
    let raw = trimmed_header(headers, "x-minipay-block-number");
    match raw {
        Some(value) => value
            .parse::<i64>()
            .map_err(|err| AppError::BadRequest(format!("invalid block number: {err}"))),
        None => Ok(0),
    }
}

fn parse_stacks_token(headers: &HeaderMap) -> Result<StacksToken, AppError> {
    let raw = trimmed_header(headers, HEADER_STACKS_TOKEN).unwrap_or("stx");
    match raw.to_ascii_lowercase().as_str() {
        "stx" => Ok(StacksToken::Stx),
        "sbtc" => Ok(StacksToken::Sbtc),
        other => Err(AppError::BadRequest(format!(
            "unknown stacks token: {other}"
        ))),
    }
}
