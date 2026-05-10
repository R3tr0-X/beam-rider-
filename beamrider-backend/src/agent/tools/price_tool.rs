use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceSnapshot {
    pub pair: String,
    pub price_usd: f64,
}

#[derive(Clone)]
pub struct PriceTool {
    client: reqwest::Client,
}

impl PriceTool {
    pub const fn new(client: reqwest::Client) -> Self {
        Self { client }
    }

    pub async fn client_ready(&self) -> bool {
        let _client = self.client.clone();
        true
    }
}
