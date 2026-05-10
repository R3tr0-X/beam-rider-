//! Read-only Hiro Stacks API client.
//!
//! BeamRider's Stacks side has two needs from this module:
//! 1. Verify a Stacks-side signal sale by tx-id, decoding the `print` event
//!    emitted by `signal-ledger.clar`.
//! 2. Read principal balances (STX or sBTC) for the off-chain rebalance
//!    decision.
//!
//! Broadcasting transactions is intentionally **not** included here. That
//! requires a signed-transactions Rust crate plus a test rig, neither of
//! which exist yet — and per AGENTS.md we do not ship live chain transaction
//! code before tests for it exist.

use chrono::Utc;
use reqwest::Url;
use serde::Deserialize;

use crate::config::StacksConfig;
use crate::domain::{
    StacksToken, VerifiedStacksSale, normalize_pair, normalize_stacks_principal,
    normalize_stacks_tx_id,
};
use crate::error::AppError;

/// `BeamRider` Stacks client. Cloneable: holds a `reqwest::Client` and a
/// pre-parsed base URL.
#[derive(Debug, Clone)]
pub struct StacksClient {
    base_url: Url,
    http: reqwest::Client,
    signal_ledger_principal: Option<String>,
}

impl StacksClient {
    pub fn from_config(config: &StacksConfig, http: reqwest::Client) -> Result<Self, AppError> {
        let base_url = Url::parse(&config.hiro_api_url)
            .map_err(|err| AppError::Config(format!("invalid stacks_hiro_url: {err}")))?;
        Ok(Self {
            base_url,
            http,
            signal_ledger_principal: config.signal_ledger_principal.clone(),
        })
    }

    pub fn signal_ledger_principal(&self) -> Option<&str> {
        self.signal_ledger_principal.as_deref()
    }

    /// Fetch a Stacks transaction by id and parse the `signal-sale` print
    /// event into a `VerifiedStacksSale`.
    ///
    /// Verification rules:
    /// - tx must be confirmed (`tx_status == "success"`).
    /// - the event must originate from the configured `signal-ledger`
    ///   contract principal.
    /// - the buyer / pair / amount in the event must match the request.
    pub async fn verify_signal_sale(
        &self,
        tx_id: &str,
        expected_buyer: &str,
        expected_pair: &str,
        token: StacksToken,
        min_amount_atoms: u128,
    ) -> Result<VerifiedStacksSale, AppError> {
        let tx_id = normalize_stacks_tx_id(tx_id).map_err(AppError::BadRequest)?;
        let buyer = normalize_stacks_principal(expected_buyer).map_err(AppError::BadRequest)?;
        let pair = normalize_pair(expected_pair).map_err(AppError::BadRequest)?;
        let ledger = self.signal_ledger_principal.as_deref().ok_or_else(|| {
            AppError::Config("STACKS_SIGNAL_LEDGER is not configured".to_string())
        })?;

        let url = self
            .base_url
            .join(&format!("/extended/v1/tx/{tx_id}"))
            .map_err(|err| AppError::External(err.to_string()))?;
        let body: HiroTxResponse = self
            .http
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        if body.tx_status != "success" {
            return Err(AppError::PaymentVerification(format!(
                "stacks tx {tx_id} status is {}",
                body.tx_status
            )));
        }

        let event = body
            .events
            .iter()
            .filter_map(parse_signal_sale_event)
            .find(|event| event.contract_id == ledger)
            .ok_or_else(|| {
                AppError::PaymentVerification(format!(
                    "stacks tx {tx_id} has no signal-sale event from {ledger}"
                ))
            })?;

        if event.buyer != buyer {
            return Err(AppError::PaymentVerification(
                "stacks signal-sale buyer mismatch".to_string(),
            ));
        }
        if !event.pair.eq_ignore_ascii_case(&pair) {
            return Err(AppError::PaymentVerification(
                "stacks signal-sale pair mismatch".to_string(),
            ));
        }
        if event.token != token {
            return Err(AppError::PaymentVerification(
                "stacks signal-sale token mismatch".to_string(),
            ));
        }
        let amount_atoms_u128 = event
            .amount
            .parse::<u128>()
            .map_err(|err| AppError::External(format!("invalid stacks event amount: {err}")))?;
        if amount_atoms_u128 < min_amount_atoms {
            return Err(AppError::PaymentVerification(format!(
                "stacks signal-sale amount {amount_atoms_u128} < required {min_amount_atoms}"
            )));
        }

        let block_height = body
            .block_height
            .or(event.block_height)
            .ok_or_else(|| AppError::External(format!("stacks tx {tx_id} has no block_height")))?;

        Ok(VerifiedStacksSale {
            buyer: event.buyer,
            pair,
            token,
            amount_atoms: event.amount,
            stacks_tx_id: tx_id,
            block_height,
            settled_at: Utc::now(),
        })
    }

    /// Read STX balance for a principal (in microSTX).
    pub async fn get_stx_balance(&self, principal: &str) -> Result<u128, AppError> {
        let principal = normalize_stacks_principal(principal).map_err(AppError::BadRequest)?;
        let url = self
            .base_url
            .join(&format!("/v2/accounts/{principal}"))
            .map_err(|err| AppError::External(err.to_string()))?;
        let body: HiroAccountResponse = self
            .http
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let balance_hex = body.balance.trim_start_matches("0x");
        u128::from_str_radix(balance_hex, 16)
            .map_err(|err| AppError::External(format!("invalid stx balance hex: {err}")))
    }
}

#[derive(Debug, Deserialize)]
struct HiroTxResponse {
    tx_status: String,
    #[serde(default)]
    block_height: Option<i64>,
    #[serde(default)]
    events: Vec<HiroEvent>,
}

#[derive(Debug, Deserialize)]
struct HiroEvent {
    #[serde(default)]
    event_type: Option<String>,
    #[serde(default)]
    contract_log: Option<HiroContractLog>,
}

#[derive(Debug, Deserialize)]
struct HiroContractLog {
    contract_id: String,
    #[serde(default)]
    value: Option<HiroValue>,
}

#[derive(Debug, Deserialize)]
struct HiroValue {
    #[serde(default)]
    repr: Option<String>,
}

#[derive(Debug, Deserialize)]
struct HiroAccountResponse {
    balance: String,
}

#[derive(Debug)]
struct ParsedSaleEvent {
    contract_id: String,
    buyer: String,
    pair: String,
    token: StacksToken,
    amount: String,
    block_height: Option<i64>,
}

/// Decode a Hiro `contract_log` event whose `repr` is the Clarity tuple
/// printed by `signal-ledger.clar::buy-signal-stx` /
/// `buy-signal-ft`.
///
/// The Clarity REPL output looks like:
///
/// ```text
/// (tuple
///   (event "signal-sale")
///   (sale-id 0x…)
///   (agent-id u1)
///   (pair u"ETH-USD")
///   (buyer 'SP…)
///   (token "stx")              ;; or the token contract principal
///   (amount u1000000)
///   (block-height u12345))
/// ```
///
/// We do not need a full Clarity parser — pulling the keyed fields out by
/// substring is sufficient and stable, and avoids pulling in a Clarity
/// dependency for one event shape.
fn parse_signal_sale_event(event: &HiroEvent) -> Option<ParsedSaleEvent> {
    let log = event.contract_log.as_ref()?;
    let event_type = event.event_type.as_deref().unwrap_or_default();
    if event_type != "smart_contract_log" && event_type != "contract_log" {
        return None;
    }
    let repr = log.value.as_ref().and_then(|value| value.repr.as_deref())?;
    if !repr.contains("\"signal-sale\"") {
        return None;
    }

    let buyer = field_value(repr, "buyer")?;
    let pair = field_value(repr, "pair")?
        .strip_prefix("u\"")?
        .strip_suffix('"')?
        .to_string();
    let token_raw = field_value(repr, "token")?;
    let token = if token_raw == "\"stx\"" {
        StacksToken::Stx
    } else if token_raw.contains("sbtc") {
        StacksToken::Sbtc
    } else {
        return None;
    };
    let amount = field_value(repr, "amount")?
        .strip_prefix('u')
        .unwrap_or_default()
        .to_string();
    let block_height_str = field_value(repr, "block-height")?
        .strip_prefix('u')
        .unwrap_or_default()
        .to_string();
    let block_height = block_height_str.parse::<i64>().ok();

    Some(ParsedSaleEvent {
        contract_id: log.contract_id.clone(),
        buyer: buyer.trim_start_matches('\'').to_string(),
        pair,
        token,
        amount,
        block_height,
    })
}

/// Pull the value associated with `key` out of a Clarity tuple repr.
/// Specifically: find `(<key> <value>)` and return `<value>` with surrounding
/// whitespace trimmed.
fn field_value<'a>(repr: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("({key} ");
    let start = repr.find(&needle)? + needle.len();
    let mut depth = 1_i64;
    let bytes = repr.as_bytes();
    let mut i = start;
    while i < bytes.len() && depth > 0 {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(repr[start..i].trim());
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_stx_signal_sale_event() {
        let repr = r#"(tuple (event "signal-sale") (sale-id 0xaa) (agent-id u1) (pair u"ETH-USD") (buyer 'SP1ABCDEFGHJKMNPQRSTVWXYZ0123456789ABCDE) (token "stx") (amount u100000) (block-height u42))"#;
        let event = HiroEvent {
            event_type: Some("smart_contract_log".to_string()),
            contract_log: Some(HiroContractLog {
                contract_id: "SP000.signal-ledger".to_string(),
                value: Some(HiroValue {
                    repr: Some(repr.to_string()),
                }),
            }),
        };

        let parsed = parse_signal_sale_event(&event).expect("event parses");
        assert_eq!(parsed.contract_id, "SP000.signal-ledger");
        assert_eq!(parsed.buyer, "SP1ABCDEFGHJKMNPQRSTVWXYZ0123456789ABCDE");
        assert_eq!(parsed.pair, "ETH-USD");
        assert_eq!(parsed.token, StacksToken::Stx);
        assert_eq!(parsed.amount, "100000");
        assert_eq!(parsed.block_height, Some(42));
    }

    #[test]
    fn parses_sbtc_signal_sale_event() {
        let repr = r#"(tuple (event "signal-sale") (sale-id 0xbb) (agent-id u2) (pair u"BTC-USD") (buyer 'SP2X) (token 'SP123.sbtc-token) (amount u500) (block-height u101))"#;
        let event = HiroEvent {
            event_type: Some("smart_contract_log".to_string()),
            contract_log: Some(HiroContractLog {
                contract_id: "SP000.signal-ledger".to_string(),
                value: Some(HiroValue {
                    repr: Some(repr.to_string()),
                }),
            }),
        };
        let parsed = parse_signal_sale_event(&event).expect("event parses");
        assert_eq!(parsed.token, StacksToken::Sbtc);
        assert_eq!(parsed.amount, "500");
    }

    #[test]
    fn rejects_non_signal_sale_event() {
        let repr = r#"(tuple (event "withdraw") (amount u1))"#;
        let event = HiroEvent {
            event_type: Some("smart_contract_log".to_string()),
            contract_log: Some(HiroContractLog {
                contract_id: "SP000.other".to_string(),
                value: Some(HiroValue {
                    repr: Some(repr.to_string()),
                }),
            }),
        };
        assert!(parse_signal_sale_event(&event).is_none());
    }
}
