use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaymentScheme {
    Exact,
}

impl PaymentScheme {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Exact => "exact",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentRequirement {
    pub scheme: String,
    pub network: String,
    pub asset: String,
    pub amount: String,
    pub pay_to: String,
    pub max_timeout_seconds: u64,
    #[serde(default)]
    pub extra: BTreeMap<String, String>,
}

impl PaymentRequirement {
    pub fn usdc_exact(
        network: impl Into<String>,
        asset: impl Into<String>,
        amount: impl Into<String>,
        pay_to: impl Into<String>,
    ) -> Self {
        let mut extra = BTreeMap::new();
        extra.insert("name".to_string(), "USDC".to_string());
        extra.insert("version".to_string(), "2".to_string());
        Self {
            scheme: PaymentScheme::Exact.as_str().to_string(),
            network: network.into(),
            asset: asset.into(),
            amount: amount.into(),
            pay_to: pay_to.into(),
            max_timeout_seconds: 60,
            extra,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentResource {
    pub url: String,
    pub description: String,
    pub mime_type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct X402PaymentPayload {
    pub x402_version: u8,
    pub accepted: PaymentRequirement,
    pub payload: Value,
    pub resource: PaymentResource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedPayment {
    pub buyer: String,
    pub chain_id: i64,
    pub network: String,
    pub token: String,
    pub amount_atoms: String,
    pub tx_hash: String,
    pub settled_at: DateTime<Utc>,
}
