//! Celo cUSD payment verifier (MiniPay surface).
//!
//! Forno's standard JSON-RPC is enough: `eth_getTransactionReceipt` plus
//! `eth_blockNumber` for confirmations, and the cUSD ERC-20 `Transfer` log
//! decoded inline. The ABI here is fixed (`Transfer(address,address,uint256)`)
//! so we do not pull in a dynamic decoder.
//!
//! This is intentionally **distinct** from x402 / CDP and is honest about
//! that boundary: cUSD on Celo is verified by reading Celo, not by trusting
//! the Coinbase facilitator (which does not support cUSD).

use chrono::Utc;
use reqwest::Client;
use serde_json::{Value, json};

use crate::config::MiniPayConfig;
use crate::domain::VerifiedPayment;
use crate::error::AppError;

/// Standard EVM signature: `keccak256("Transfer(address,address,uint256)")`.
pub const ERC20_TRANSFER_TOPIC: &str =
    "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";

#[derive(Debug, Clone)]
pub struct CeloPaymentVerifier {
    config: MiniPayConfig,
    client: Client,
}

impl CeloPaymentVerifier {
    pub fn new(config: MiniPayConfig, client: Client) -> Self {
        Self { config, client }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub fn cusd_address(&self) -> &str {
        &self.config.cusd_address
    }

    /// Verify a Celo cUSD payment by tx hash.
    ///
    /// Returns a `VerifiedPayment` whose `network` is `eip155:42220`,
    /// `token` is the cUSD address, `amount_atoms` is the decimal-string
    /// `value` field of the Transfer log (1e18 scale), and `tx_hash` is
    /// the lower-cased input.
    pub async fn verify(&self, tx_hash: &str) -> Result<VerifiedPayment, AppError> {
        if !self.config.enabled {
            return Err(AppError::PaymentVerification(
                "MiniPay verifier is not enabled".to_string(),
            ));
        }
        let tx_hash = normalize_tx_hash(tx_hash)?;
        let receipt = self.fetch_receipt(&tx_hash).await?;
        let confirmed_block = self.parse_confirmed_block(&receipt)?;
        if self.config.min_confirmations > 0 {
            let latest = self.fetch_block_number().await?;
            if latest.saturating_sub(confirmed_block) < self.config.min_confirmations {
                return Err(AppError::PaymentVerification(format!(
                    "minipay tx {tx_hash} not yet confirmed (block {confirmed_block} of latest {latest})"
                )));
            }
        }

        let transfer = decode_transfer(&receipt, &self.config.cusd_address, &self.config.receiver)?;
        let min = parse_amount(&self.config.min_amount_atoms, "configured min amount")?;
        if transfer.value < min {
            return Err(AppError::PaymentVerification(format!(
                "minipay amount {} < required {}",
                transfer.value, min
            )));
        }

        Ok(VerifiedPayment {
            buyer: lower_address(&transfer.from),
            chain_id: 42220,
            network: "eip155:42220".to_string(),
            token: lower_address(&self.config.cusd_address),
            amount_atoms: transfer.value.to_string(),
            tx_hash,
            settled_at: Utc::now(),
        })
    }

    async fn fetch_receipt(&self, tx_hash: &str) -> Result<Value, AppError> {
        let response: JsonRpcResponse = self
            .client
            .post(&self.config.forno_url)
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "eth_getTransactionReceipt",
                "params": [tx_hash],
                "id": 1,
            }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        if let Some(err) = response.error {
            return Err(AppError::External(format!("forno error: {err}")));
        }
        response
            .result
            .filter(|value| !value.is_null())
            .ok_or_else(|| AppError::PaymentVerification(format!("no receipt for {tx_hash}")))
    }

    async fn fetch_block_number(&self) -> Result<u64, AppError> {
        let response: JsonRpcResponse = self
            .client
            .post(&self.config.forno_url)
            .json(&json!({
                "jsonrpc": "2.0",
                "method": "eth_blockNumber",
                "params": [],
                "id": 1,
            }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        if let Some(err) = response.error {
            return Err(AppError::External(format!("forno error: {err}")));
        }
        let raw = response
            .result
            .and_then(|value| value.as_str().map(str::to_string))
            .ok_or_else(|| AppError::External("missing eth_blockNumber result".to_string()))?;
        parse_hex_u64(&raw, "eth_blockNumber")
    }

    fn parse_confirmed_block(&self, receipt: &Value) -> Result<u64, AppError> {
        let status = receipt.get("status").and_then(Value::as_str).unwrap_or("");
        if status != "0x1" {
            return Err(AppError::PaymentVerification(format!(
                "minipay tx status is {status}"
            )));
        }
        let block = receipt
            .get("blockNumber")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::External("receipt is missing blockNumber".to_string()))?;
        parse_hex_u64(block, "blockNumber")
    }
}

#[derive(Debug, serde::Deserialize)]
struct JsonRpcResponse {
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<Value>,
}

#[derive(Debug)]
struct DecodedTransfer {
    from: String,
    value: u128,
}

fn decode_transfer(
    receipt: &Value,
    expected_token: &str,
    expected_to: &str,
) -> Result<DecodedTransfer, AppError> {
    let logs = receipt
        .get("logs")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::External("receipt missing logs".to_string()))?;
    let token_lower = expected_token.to_ascii_lowercase();
    let to_lower = expected_to.to_ascii_lowercase();

    for log in logs {
        let address = log
            .get("address")
            .and_then(Value::as_str)
            .map(str::to_ascii_lowercase)
            .unwrap_or_default();
        if address != token_lower {
            continue;
        }
        let topics = log
            .get("topics")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if topics.len() != 3 {
            continue;
        }
        let topic0 = topics[0].as_str().unwrap_or_default();
        if !topic0.eq_ignore_ascii_case(ERC20_TRANSFER_TOPIC) {
            continue;
        }
        let from = topic_to_address(topics[1].as_str().unwrap_or_default())?;
        let to = topic_to_address(topics[2].as_str().unwrap_or_default())?;
        if !to.eq_ignore_ascii_case(&to_lower) {
            continue;
        }
        let data = log
            .get("data")
            .and_then(Value::as_str)
            .ok_or_else(|| AppError::External("transfer log missing data".to_string()))?;
        let value = parse_uint256(data)?;
        return Ok(DecodedTransfer { from, value });
    }
    Err(AppError::PaymentVerification(format!(
        "no cUSD Transfer log to {to_lower} found"
    )))
}

fn topic_to_address(topic: &str) -> Result<String, AppError> {
    let trimmed = topic.strip_prefix("0x").unwrap_or(topic);
    if trimmed.len() != 64 || !trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(AppError::External(format!("invalid topic: {topic}")));
    }
    Ok(format!("0x{}", &trimmed[24..]))
}

fn parse_uint256(data: &str) -> Result<u128, AppError> {
    let trimmed = data.strip_prefix("0x").unwrap_or(data);
    if trimmed.len() != 64 || !trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(AppError::External(format!("invalid uint256 data: {data}")));
    }
    let high = u128::from_str_radix(&trimmed[..32], 16)
        .map_err(|err| AppError::External(format!("uint256 high: {err}")))?;
    if high != 0 {
        return Err(AppError::External(
            "transfer value exceeds u128".to_string(),
        ));
    }
    u128::from_str_radix(&trimmed[32..], 16)
        .map_err(|err| AppError::External(format!("uint256 low: {err}")))
}

fn parse_hex_u64(value: &str, label: &str) -> Result<u64, AppError> {
    let hex = value.strip_prefix("0x").unwrap_or(value);
    u64::from_str_radix(hex, 16)
        .map_err(|err| AppError::External(format!("invalid {label}: {err}")))
}

fn parse_amount(value: &str, label: &str) -> Result<u128, AppError> {
    value
        .parse::<u128>()
        .map_err(|err| AppError::Config(format!("invalid {label}: {err}")))
}

fn normalize_tx_hash(value: &str) -> Result<String, AppError> {
    let trimmed = value.trim();
    let hex = trimmed.strip_prefix("0x").unwrap_or(trimmed);
    if hex.len() != 64 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(AppError::BadRequest(
            "tx hash must be 32 hex bytes".to_string(),
        ));
    }
    Ok(format!("0x{}", hex.to_ascii_lowercase()))
}

fn lower_address(value: &str) -> String {
    value.to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn make_receipt(amount_hex: &str) -> Value {
        json!({
            "status": "0x1",
            "blockNumber": "0x10",
            "logs": [{
                "address": "0x765de816845861e75a25fca122bb6898b8b1282a",
                "topics": [
                    ERC20_TRANSFER_TOPIC,
                    "0x000000000000000000000000aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                    "0x000000000000000000000000bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
                ],
                "data": amount_hex,
            }]
        })
    }

    #[test]
    fn decodes_valid_transfer_log() {
        let amount_hex = format!("0x{:0>64x}", 1_000_000_000_000_000_000_u128);
        let receipt = make_receipt(&amount_hex);

        let decoded = decode_transfer(
            &receipt,
            "0x765DE816845861e75A25fCA122bb6898B8B1282a",
            "0xbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        )
        .unwrap();
        assert_eq!(decoded.from, "0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        assert_eq!(decoded.value, 1_000_000_000_000_000_000);
    }

    #[test]
    fn rejects_log_with_wrong_recipient() {
        let amount_hex = format!("0x{:0>64x}", 1_u128);
        let receipt = make_receipt(&amount_hex);
        let err = decode_transfer(
            &receipt,
            "0x765DE816845861e75A25fCA122bb6898B8B1282a",
            "0x0000000000000000000000000000000000000001",
        )
        .unwrap_err();
        assert!(matches!(err, AppError::PaymentVerification(_)));
    }

    #[test]
    fn parses_uint256_data_and_rejects_overflow() {
        let small = format!("0x{:0>64x}", 42_u128);
        assert_eq!(parse_uint256(&small).unwrap(), 42);

        let too_big = format!("0x{:0>32x}{:0>32x}", 1_u128, 0_u128);
        assert!(parse_uint256(&too_big).is_err());
    }
}
