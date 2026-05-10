use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::{
    RebalanceStatus, SIGNATURE_SCHEME, SignalKind, SignatureEnvelope, SignedResponse, Venue,
};
use crate::repositories::{StoredRebalance, StoredSignal};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub service: String,
    pub database: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalResponse {
    pub pair: String,
    pub kind: SignalKind,
    pub value_bps: i64,
    pub confidence: f64,
    pub created_at: DateTime<Utc>,
    pub signature_scheme: String,
    pub signature: String,
    pub public_key: String,
    pub prompt_version: String,
}

impl From<SignedResponse> for SignalResponse {
    fn from(value: SignedResponse) -> Self {
        Self {
            pair: value.signal.pair,
            kind: value.signal.kind,
            value_bps: value.signal.value_bps,
            confidence: value.signal.confidence.value(),
            created_at: value.signal.created_at,
            signature_scheme: value.attestation.scheme,
            signature: value.attestation.signature_hex,
            public_key: value.attestation.public_key_hex,
            prompt_version: value.prompt_version,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredSignalResponse {
    pub id: i64,
    pub signal: SignalResponse,
}

impl StoredSignalResponse {
    pub fn from_stored(value: StoredSignal, public_key: &[u8]) -> Self {
        let attestation = SignatureEnvelope {
            scheme: SIGNATURE_SCHEME.to_string(),
            signature_hex: hex::encode(value.signature),
            public_key_hex: hex::encode(public_key),
        };
        Self {
            id: value.id,
            signal: SignalResponse::from(SignedResponse {
                signal: value.signal,
                attestation,
                prompt_version: "beamrider-signal-v1".to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeResponse {
    pub input: String,
    pub digest: String,
    pub created_at: DateTime<Utc>,
    pub attestation: SignatureEnvelope,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebalanceSummary {
    pub id: i64,
    pub src_chain: i64,
    pub dest_chain: i64,
    pub amount_usdc_atoms: String,
    pub venue: Venue,
    pub expected_apy_bps: i64,
    pub status: RebalanceStatus,
    pub proposed_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

impl From<StoredRebalance> for RebalanceSummary {
    fn from(value: StoredRebalance) -> Self {
        Self {
            id: value.id,
            src_chain: value.plan.src_chain,
            dest_chain: value.plan.dest_chain,
            amount_usdc_atoms: value.plan.amount_usdc_atoms,
            venue: value.plan.venue,
            expected_apy_bps: value.plan.expected_apy_bps,
            status: value.plan.status,
            proposed_at: value.plan.proposed_at,
            finished_at: value.finished_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStatusResponse {
    pub service: String,
    pub signal_count: i64,
    pub latest_signal: Option<StoredSignalResponse>,
    pub rebalance_count: i64,
    pub latest_rebalance: Option<RebalanceSummary>,
    #[serde(default)]
    pub stacks_sale_count: i64,
    #[serde(default)]
    pub minipay_payment_count: i64,
    #[serde(default)]
    pub active_session_count: i64,
    pub timestamp: DateTime<Utc>,
}
