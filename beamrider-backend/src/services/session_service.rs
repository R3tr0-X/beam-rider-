//! x402 V2 session voucher issuance and consumption.
//!
//! A buyer pays once via the existing `X402Verifier`, the agent records the
//! settlement in the `sessions` table with a freshly-minted opaque token,
//! and subsequent signal calls present that token via the `X402-Session`
//! header instead of paying again.
//!
//! The token is **opaque** — 32 random bytes encoded base64-url. We treat
//! it as an unguessable bearer credential. There is no signature on the
//! token: the database row is the source of truth.

use chrono::{Duration, Utc};
use rand::TryRngCore;
use rand::rngs::OsRng;

use crate::config::SessionConfig;
use crate::domain::VerifiedPayment;
use crate::error::AppError;
use crate::repositories::{SqliteSessionRepository, StoredSession};

#[derive(Debug, Clone)]
pub struct IssuedSession {
    pub token: String,
    pub balance: i64,
    pub expiry: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct SessionService {
    config: SessionConfig,
    repo: SqliteSessionRepository,
}

impl SessionService {
    pub const fn new(config: SessionConfig, repo: SqliteSessionRepository) -> Self {
        Self { config, repo }
    }

    /// Issue a new session voucher backed by an x402 payment. The number of
    /// requests is clamped to `1..=128` to keep operator exposure bounded.
    pub async fn issue(
        &self,
        payment: &VerifiedPayment,
        requests: Option<i64>,
    ) -> Result<IssuedSession, AppError> {
        let balance = requests
            .unwrap_or(self.config.default_requests)
            .clamp(1, 128);
        let expiry = Utc::now() + Duration::seconds(self.config.ttl_seconds.max(60));
        let token = mint_token();

        let session = StoredSession {
            token: token.clone(),
            buyer: payment.buyer.to_ascii_lowercase(),
            chain_id: payment.chain_id,
            paid_token: payment.token.to_ascii_lowercase(),
            paid_amount: payment.amount_atoms.clone(),
            balance,
            requests_used: 0,
            expiry,
            settle_tx_hash: payment.tx_hash.clone(),
        };
        self.repo.insert(&session).await?;
        Ok(IssuedSession {
            token,
            balance,
            expiry,
        })
    }

    /// Consume one credit from a presented session token. Returns a
    /// `VerifiedPayment`-shaped domain value the signal pipeline can record
    /// uniformly across payment surfaces.
    pub async fn consume(&self, token: &str) -> Result<VerifiedPayment, AppError> {
        if token.is_empty() {
            return Err(AppError::PaymentVerification(
                "empty session token".to_string(),
            ));
        }
        let session = self.repo.consume_one(token).await?;
        let used_index = session.requests_used;
        Ok(VerifiedPayment {
            buyer: session.buyer,
            chain_id: session.chain_id,
            network: format!("session:eip155:{}", session.chain_id),
            token: session.paid_token,
            amount_atoms: session.paid_amount,
            tx_hash: format!("{}#{}", session.settle_tx_hash, used_index),
            settled_at: Utc::now(),
        })
    }

    pub async fn count_active(&self) -> Result<i64, AppError> {
        self.repo.count_active().await
    }
}

fn mint_token() -> String {
    let mut bytes = [0_u8; 32];
    OsRng
        .try_fill_bytes(&mut bytes)
        .expect("os rng must produce randomness");
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes)
}
