use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

pub const MAX_PAIR_LEN: usize = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SignalKind {
    Buy,
    Sell,
    Hold,
}

impl SignalKind {
    pub const fn as_i64(self) -> i64 {
        match self {
            Self::Buy => 0,
            Self::Sell => 1,
            Self::Hold => 2,
        }
    }

    pub const fn canonical_name(self) -> &'static str {
        match self {
            Self::Buy => "BUY",
            Self::Sell => "SELL",
            Self::Hold => "HOLD",
        }
    }

    pub fn from_i64(value: i64) -> Option<Self> {
        match value {
            0 => Some(Self::Buy),
            1 => Some(Self::Sell),
            2 => Some(Self::Hold),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Confidence(f64);

impl Confidence {
    pub fn new(value: f64) -> Result<Self, String> {
        if value.is_finite() && (0.0..=1.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err("confidence must be finite and between 0.0 and 1.0".to_string())
        }
    }

    pub const fn value(self) -> f64 {
        self.0
    }

    pub fn ppm(self) -> i64 {
        (self.0 * 1_000_000.0).round() as i64
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MarketSignal {
    pub pair: String,
    pub kind: SignalKind,
    pub value_bps: i64,
    pub confidence: Confidence,
    pub created_at: DateTime<Utc>,
}

impl MarketSignal {
    pub fn new(
        pair: impl AsRef<str>,
        kind: SignalKind,
        value_bps: i64,
        confidence: f64,
        created_at: DateTime<Utc>,
    ) -> Result<Self, String> {
        Ok(Self {
            pair: normalize_pair(pair.as_ref())?,
            kind,
            value_bps,
            confidence: Confidence::new(confidence)?,
            created_at,
        })
    }

    pub fn canonical_bytes(&self) -> Vec<u8> {
        format!(
            "beamrider.signal.v1\npair={}\nkind={}\nvalue_bps={}\nconfidence_ppm={}\ncreated_at={}",
            self.pair,
            self.kind.canonical_name(),
            self.value_bps,
            self.confidence.ppm(),
            self.created_at.to_rfc3339_opts(SecondsFormat::Millis, true)
        )
        .into_bytes()
    }
}

pub fn normalize_pair(input: &str) -> Result<String, String> {
    let pair = input.trim().to_ascii_uppercase();
    if pair.is_empty() {
        return Err("pair is required".to_string());
    }
    if pair.len() > MAX_PAIR_LEN {
        return Err(format!("pair must be at most {MAX_PAIR_LEN} characters"));
    }

    let mut parts = pair.split('-');
    let base = parts.next().unwrap_or_default();
    let quote = parts.next().unwrap_or_default();
    if parts.next().is_some() || base.is_empty() || quote.is_empty() {
        return Err("pair must use BASE-QUOTE format".to_string());
    }
    if base.len() > 10 || quote.len() > 10 {
        return Err("pair symbols must be at most 10 characters each".to_string());
    }
    if !base.chars().all(|c| c.is_ascii_alphanumeric())
        || !quote.chars().all(|c| c.is_ascii_alphanumeric())
    {
        return Err("pair symbols must be ASCII alphanumeric".to_string());
    }
    Ok(pair)
}
