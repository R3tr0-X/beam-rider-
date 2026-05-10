use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalRequest {
    pub pair: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeRequest {
    pub input: String,
    #[serde(default)]
    pub context: Option<Value>,
}
