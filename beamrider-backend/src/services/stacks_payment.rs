//! Service-layer wrapper that turns a Hiro-verified `signal-ledger` event
//! into a domain `VerifiedStacksSale`.
//!
//! The wrapper exists so handlers don't need to know about the Hiro client
//! shape and can stay thin per AGENTS.md.

use crate::chains::stacks::StacksClient;
use crate::config::StacksConfig;
use crate::domain::{StacksToken, VerifiedStacksSale};
use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct StacksPaymentVerifier {
    client: StacksClient,
    enabled: bool,
}

impl StacksPaymentVerifier {
    pub fn new(config: &StacksConfig, client: StacksClient) -> Self {
        Self {
            client,
            enabled: config.enabled && config.signal_ledger_principal.is_some(),
        }
    }

    pub const fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub async fn verify(
        &self,
        tx_id: &str,
        buyer: &str,
        pair: &str,
        token: StacksToken,
        min_amount_atoms: u128,
    ) -> Result<VerifiedStacksSale, AppError> {
        if !self.enabled {
            return Err(AppError::PaymentVerification(
                "stacks payment verifier is not enabled".to_string(),
            ));
        }
        self.client
            .verify_signal_sale(tx_id, buyer, pair, token, min_amount_atoms)
            .await
    }
}
