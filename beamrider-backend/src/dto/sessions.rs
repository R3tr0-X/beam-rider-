use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize)]
pub struct IssueSessionRequest {
    /// Number of signal calls this voucher buys. Server clamps to a sane range.
    #[serde(default)]
    pub requests: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueSessionResponse {
    pub token: String,
    pub balance: i64,
    pub expiry: DateTime<Utc>,
}
