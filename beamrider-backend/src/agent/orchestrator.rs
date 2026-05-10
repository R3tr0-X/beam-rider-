use chrono::Utc;
use rig::{client::CompletionClient, completion::Prompt, providers::gemini};
use serde::Deserialize;

use crate::agent::prompts::SIGNAL_SYSTEM_PROMPT;
use crate::domain::{MarketSignal, SignalKind, normalize_pair};
use crate::error::AppError;
use crate::repositories::StoredSignal;

#[derive(Debug, Clone)]
pub struct GeminiAgentConfig {
    pub api_key: String,
    pub model: String,
}

#[derive(Clone)]
pub struct SignalAgent {
    gemini: Option<GeminiAgentConfig>,
    http_client: reqwest::Client,
}

impl SignalAgent {
    pub fn new(gemini: Option<GeminiAgentConfig>, http_client: reqwest::Client) -> Self {
        Self {
            gemini,
            http_client,
        }
    }

    pub async fn decide_signal(
        &self,
        pair: &str,
        history: &[StoredSignal],
    ) -> Result<MarketSignal, AppError> {
        let pair = normalize_pair(pair).map_err(AppError::BadRequest)?;

        if let Some(config) = &self.gemini {
            match self.decide_with_gemini(config, &pair, history).await {
                Ok(signal) => return Ok(signal),
                Err(err) => {
                    tracing::warn!(error = %err, "Gemini signal generation failed; using deterministic fallback");
                }
            }
        }

        deterministic_signal(&pair, history)
    }

    async fn decide_with_gemini(
        &self,
        config: &GeminiAgentConfig,
        pair: &str,
        history: &[StoredSignal],
    ) -> Result<MarketSignal, AppError> {
        let client = gemini::Client::builder(&config.api_key)
            .custom_client(self.http_client.clone())
            .build()
            .map_err(|err| AppError::External(err.to_string()))?;
        let agent = client
            .agent(&config.model)
            .preamble(SIGNAL_SYSTEM_PROMPT)
            .temperature(0.0)
            .build();
        let response = agent
            .prompt(prompt_for_pair(pair, history))
            .await
            .map_err(|err| AppError::External(err.to_string()))?;
        let decision = parse_decision(&response)?;
        MarketSignal::new(
            pair,
            decision.kind,
            decision.value_bps,
            decision.confidence,
            Utc::now(),
        )
        .map_err(AppError::BadRequest)
    }
}

#[derive(Debug, Deserialize)]
struct GeminiDecision {
    kind: SignalKindText,
    value_bps: i64,
    confidence: f64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum SignalKindText {
    Buy,
    Sell,
    Hold,
}

impl From<SignalKindText> for SignalKind {
    fn from(value: SignalKindText) -> Self {
        match value {
            SignalKindText::Buy => Self::Buy,
            SignalKindText::Sell => Self::Sell,
            SignalKindText::Hold => Self::Hold,
        }
    }
}

fn parse_decision(response: &str) -> Result<ParsedDecision, AppError> {
    let json = extract_json_object(response)
        .ok_or_else(|| AppError::External("Gemini response did not contain JSON".to_string()))?;
    let decision: GeminiDecision =
        serde_json::from_str(json).map_err(|err| AppError::External(err.to_string()))?;
    Ok(ParsedDecision {
        kind: decision.kind.into(),
        value_bps: decision.value_bps.clamp(-10_000, 10_000),
        confidence: decision.confidence,
    })
}

#[derive(Debug)]
struct ParsedDecision {
    kind: SignalKind,
    value_bps: i64,
    confidence: f64,
}

fn extract_json_object(response: &str) -> Option<&str> {
    let start = response.find('{')?;
    let end = response.rfind('}')?;
    (start <= end).then_some(&response[start..=end])
}

fn prompt_for_pair(pair: &str, history: &[StoredSignal]) -> String {
    let compact_history = history
        .iter()
        .take(8)
        .map(|stored| {
            format!(
                "{}:{}:{}:{}",
                stored.signal.created_at.to_rfc3339(),
                stored.signal.kind.canonical_name(),
                stored.signal.value_bps,
                stored.signal.confidence.ppm()
            )
        })
        .collect::<Vec<_>>()
        .join("|");

    format!("pair={pair}\nrecent_history={compact_history}")
}

fn deterministic_signal(pair: &str, history: &[StoredSignal]) -> Result<MarketSignal, AppError> {
    let mut seed = fnv1a(pair.as_bytes());
    for stored in history.iter().take(16) {
        seed ^= stored.signal.value_bps.unsigned_abs();
        seed = seed.rotate_left(7) ^ stored.signal.kind.as_i64() as u64;
    }

    let bucket = seed % 3;
    let kind = match bucket {
        0 => SignalKind::Buy,
        1 => SignalKind::Sell,
        _ => SignalKind::Hold,
    };
    let raw_bps = 25 + (seed % 226) as i64;
    let value_bps = match kind {
        SignalKind::Buy => raw_bps,
        SignalKind::Sell => -raw_bps,
        SignalKind::Hold => 0,
    };
    let confidence = 0.55 + ((seed >> 8) % 3500) as f64 / 10_000.0;

    MarketSignal::new(pair, kind, value_bps, confidence.min(0.90), Utc::now())
        .map_err(AppError::BadRequest)
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
