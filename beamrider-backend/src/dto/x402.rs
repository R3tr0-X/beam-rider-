use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::{PaymentRequirement, PaymentResource};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentRequiredResponse {
    pub x402_version: u8,
    pub accepts: Vec<PaymentRequirement>,
    pub resource: PaymentResource,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CdpVerifyRequest {
    pub x402_version: u8,
    pub payment_payload: Value,
    pub payment_requirements: PaymentRequirement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CdpVerifyResponse {
    pub is_valid: bool,
    pub payer: String,
    #[serde(default)]
    pub invalid_reason: Option<String>,
    #[serde(default)]
    pub invalid_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixturePaymentHeader {
    pub buyer: String,
    pub chain_id: i64,
    pub network: String,
    pub token: String,
    pub amount_atoms: String,
    pub tx_hash: String,
}
