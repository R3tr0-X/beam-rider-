use async_trait::async_trait;
use axum::http::HeaderMap;
use base64::Engine;
use chrono::Utc;
use serde_json::Value;

use crate::config::X402Config;
use crate::domain::{PaymentRequirement, VerifiedPayment};
use crate::dto::{
    CdpVerifyRequest, CdpVerifyResponse, FixturePaymentHeader, PaymentRequiredResponse,
};
use crate::error::AppError;

const PAYMENT_HEADERS: [&str; 3] = ["x-payment", "payment-signature", "payment-response"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum X402VerifierMode {
    Cdp,
    Fixture,
}

#[derive(Clone)]
pub struct X402Verifier {
    config: X402Config,
    client: reqwest::Client,
    mode: X402VerifierMode,
}

impl X402Verifier {
    pub fn cdp(config: X402Config, client: reqwest::Client) -> Self {
        Self {
            config,
            client,
            mode: X402VerifierMode::Cdp,
        }
    }

    pub fn fixture(config: X402Config, client: reqwest::Client) -> Self {
        Self {
            config,
            client,
            mode: X402VerifierMode::Fixture,
        }
    }

    pub fn payment_required(&self, pair: &str) -> AppError {
        AppError::PaymentRequired(Box::new(PaymentRequiredResponse {
            x402_version: 2,
            accepts: self.config.payment_requirements(),
            resource: self.config.resource(pair),
            error: "x402 payment required".to_string(),
        }))
    }

    pub async fn verify_headers(
        &self,
        headers: &HeaderMap,
        pair: &str,
    ) -> Result<VerifiedPayment, AppError> {
        let raw = payment_header(headers).ok_or_else(|| self.payment_required(pair))?;
        match self.mode {
            X402VerifierMode::Fixture => self.verify_fixture(raw),
            X402VerifierMode::Cdp => self.verify_cdp(raw).await,
        }
    }

    fn verify_fixture(&self, raw: &str) -> Result<VerifiedPayment, AppError> {
        let value = decode_header_payload(raw)?;
        let fixture: FixturePaymentHeader =
            serde_json::from_value(value).map_err(|err| AppError::BadRequest(err.to_string()))?;
        let expected_chain = self
            .config
            .chain_id_for_network(&fixture.network)
            .ok_or_else(|| AppError::PaymentVerification("unsupported x402 network".to_string()))?;
        let required = self
            .config
            .payment_requirements()
            .into_iter()
            .find(|requirement| requirement.network == fixture.network)
            .ok_or_else(|| AppError::PaymentVerification("unsupported x402 network".to_string()))?;
        if expected_chain != fixture.chain_id {
            return Err(AppError::PaymentVerification(
                "x402 fixture chain_id/network mismatch".to_string(),
            ));
        }
        if !required.asset.eq_ignore_ascii_case(&fixture.token) {
            return Err(AppError::PaymentVerification(
                "x402 fixture token mismatch".to_string(),
            ));
        }
        if fixture.tx_hash.trim().is_empty() {
            return Err(AppError::PaymentVerification(
                "x402 fixture tx_hash is required".to_string(),
            ));
        }
        if amount_lt(&fixture.amount_atoms, &self.config.amount_atoms)? {
            return Err(AppError::PaymentVerification(
                "x402 fixture amount is below required amount".to_string(),
            ));
        }
        Ok(VerifiedPayment {
            buyer: fixture.buyer,
            chain_id: fixture.chain_id,
            network: fixture.network,
            token: fixture.token,
            amount_atoms: fixture.amount_atoms,
            tx_hash: fixture.tx_hash,
            settled_at: Utc::now(),
        })
    }

    async fn verify_cdp(&self, raw: &str) -> Result<VerifiedPayment, AppError> {
        let bearer = self.config.bearer_token.as_deref().ok_or_else(|| {
            AppError::PaymentVerification(
                "X402_FACILITATOR_BEARER_TOKEN is not configured".to_string(),
            )
        })?;
        let payment_payload = decode_header_payload(raw)?;
        let accepted: PaymentRequirement =
            serde_json::from_value(required_field(&payment_payload, "accepted")?.clone())
                .map_err(|err| AppError::PaymentVerification(err.to_string()))?;
        self.validate_requirement(&accepted)?;

        let response = self
            .client
            .post(&self.config.facilitator_url)
            .bearer_auth(bearer)
            .json(&CdpVerifyRequest {
                x402_version: 2,
                payment_payload: payment_payload.clone(),
                payment_requirements: accepted.clone(),
            })
            .send()
            .await?
            .error_for_status()?
            .json::<CdpVerifyResponse>()
            .await?;

        if !response.is_valid {
            let reason = response
                .invalid_message
                .or(response.invalid_reason)
                .unwrap_or_else(|| "facilitator rejected payment".to_string());
            return Err(AppError::PaymentVerification(reason));
        }

        let chain_id = self
            .config
            .chain_id_for_network(&accepted.network)
            .ok_or_else(|| AppError::PaymentVerification("unsupported x402 network".to_string()))?;

        Ok(VerifiedPayment {
            buyer: response.payer,
            chain_id,
            network: accepted.network,
            token: accepted.asset,
            amount_atoms: accepted.amount,
            tx_hash: synthetic_payment_id(&payment_payload),
            settled_at: Utc::now(),
        })
    }

    fn validate_requirement(&self, accepted: &PaymentRequirement) -> Result<(), AppError> {
        let configured = self.config.payment_requirements();
        let matches = configured.iter().any(|required| {
            required.scheme == accepted.scheme
                && required.network == accepted.network
                && required.asset.eq_ignore_ascii_case(&accepted.asset)
                && required.pay_to.eq_ignore_ascii_case(&accepted.pay_to)
                && !amount_lt(&accepted.amount, &required.amount).unwrap_or(true)
        });
        if matches {
            Ok(())
        } else {
            Err(AppError::PaymentVerification(
                "payment requirements do not match BeamRider price".to_string(),
            ))
        }
    }
}

#[async_trait]
pub trait CeloPaymentVerifier: Send + Sync {
    async fn verify_celo_payment(&self, _payload: &Value) -> Result<VerifiedPayment, AppError>;
}

#[derive(Debug, Clone, Default)]
pub struct UnsupportedCeloPaymentVerifier;

#[async_trait]
impl CeloPaymentVerifier for UnsupportedCeloPaymentVerifier {
    async fn verify_celo_payment(&self, _payload: &Value) -> Result<VerifiedPayment, AppError> {
        Err(AppError::PaymentVerification(
            "Celo cUSD x402 verifier is not implemented in the MVP".to_string(),
        ))
    }
}

fn payment_header(headers: &HeaderMap) -> Option<&str> {
    for name in PAYMENT_HEADERS {
        if let Some(value) = headers.get(name).and_then(|value| value.to_str().ok()) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed);
            }
        }
    }
    None
}

pub fn decode_header_payload(raw: &str) -> Result<Value, AppError> {
    let payload = raw.strip_prefix("fixture:").unwrap_or(raw).trim();
    if payload.starts_with('{') {
        return serde_json::from_str(payload).map_err(|err| AppError::BadRequest(err.to_string()));
    }

    let decoded = base64::engine::general_purpose::STANDARD
        .decode(payload)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload))
        .map_err(|err| AppError::BadRequest(format!("invalid x402 payment header: {err}")))?;
    serde_json::from_slice(&decoded).map_err(|err| AppError::BadRequest(err.to_string()))
}

fn required_field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, AppError> {
    value
        .get(key)
        .ok_or_else(|| AppError::PaymentVerification(format!("payment payload missing {key}")))
}

fn amount_lt(left: &str, right: &str) -> Result<bool, AppError> {
    let left = left
        .parse::<u128>()
        .map_err(|err| AppError::BadRequest(format!("invalid amount: {err}")))?;
    let right = right
        .parse::<u128>()
        .map_err(|err| AppError::BadRequest(format!("invalid amount: {err}")))?;
    Ok(left < right)
}

fn synthetic_payment_id(payload: &Value) -> String {
    payload
        .pointer("/payload/authorization/nonce")
        .and_then(Value::as_str)
        .or_else(|| {
            payload
                .pointer("/payload/signature")
                .and_then(Value::as_str)
        })
        .map(ToString::to_string)
        .unwrap_or_else(|| format!("x402:{}", hex::encode(payload.to_string())))
}
