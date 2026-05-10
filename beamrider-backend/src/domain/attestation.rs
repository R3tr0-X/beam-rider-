use serde::{Deserialize, Serialize};

use super::signal::MarketSignal;

pub const SIGNATURE_SCHEME: &str = "ed25519";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignatureEnvelope {
    pub scheme: String,
    pub signature_hex: String,
    pub public_key_hex: String,
}

impl SignatureEnvelope {
    pub fn new(signature: &[u8], public_key: &[u8]) -> Self {
        Self {
            scheme: SIGNATURE_SCHEME.to_string(),
            signature_hex: hex::encode(signature),
            public_key_hex: hex::encode(public_key),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignedResponse {
    pub signal: MarketSignal,
    pub attestation: SignatureEnvelope,
    pub prompt_version: String,
}

impl SignedResponse {
    pub fn new(signal: MarketSignal, signature: &[u8], public_key: &[u8]) -> Self {
        Self {
            signal,
            attestation: SignatureEnvelope::new(signature, public_key),
            prompt_version: "beamrider-signal-v1".to_string(),
        }
    }
}
