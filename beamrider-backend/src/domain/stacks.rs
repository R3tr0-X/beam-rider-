use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Token paid on the Stacks side. Kept as a closed enum rather than a free
/// string so the verifier and the repository can't drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StacksToken {
    Stx,
    Sbtc,
}

impl StacksToken {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Stx => "stx",
            Self::Sbtc => "sbtc",
        }
    }

    pub fn parse_db(value: &str) -> Option<Self> {
        match value {
            "stx" => Some(Self::Stx),
            "sbtc" => Some(Self::Sbtc),
            _ => None,
        }
    }
}

/// A verified Stacks-side signal sale. Mirrors `VerifiedPayment` but is a
/// distinct type because Stacks has no EIP-155 `chain_id` and the on-wire
/// principal format is unrelated to EVM addresses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedStacksSale {
    pub buyer: String,
    pub pair: String,
    pub token: StacksToken,
    pub amount_atoms: String,
    pub stacks_tx_id: String,
    pub block_height: i64,
    pub settled_at: DateTime<Utc>,
}

/// `principal` validation is intentionally minimal — Hiro is the source of
/// truth. We only reject obvious garbage so an attacker can't smuggle SQL
/// metacharacters through the buyer field.
pub fn normalize_stacks_principal(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err("stacks principal is required".to_string());
    }
    if trimmed.len() > 96 {
        return Err("stacks principal too long".to_string());
    }
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
    {
        return Err("stacks principal must be alphanumeric with optional . or -".to_string());
    }
    Ok(trimmed.to_string())
}

/// Validate a Stacks tx id: 0x-prefixed 32-byte hex.
pub fn normalize_stacks_tx_id(value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    let hex = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    if hex.len() != 64 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("stacks tx id must be 32 hex bytes (0x-prefixed allowed)".to_string());
    }
    Ok(format!("0x{}", hex.to_ascii_lowercase()))
}
