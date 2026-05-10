use crate::agent::SignalAgent;
use crate::crypto::ResponseSigner;
use crate::domain::{SignedResponse, VerifiedPayment, VerifiedStacksSale, normalize_pair};
use crate::error::AppError;
use crate::repositories::{
    SqliteMiniPayRepository, SqliteSaleRepository, SqliteSignalRepository,
    SqliteStacksSaleRepository,
};

/// Discriminator for the upstream payment surface — uniform inside the
/// service so the handler stays thin and the persistence call is one match.
#[derive(Debug, Clone)]
pub enum SignalPayment {
    /// x402 USDC on Base / Arbitrum (CDP-verified) or session-token consume.
    X402(VerifiedPayment),
    /// Celo cUSD verified directly via Forno.
    MiniPay {
        payment: VerifiedPayment,
        block_number: i64,
    },
    /// STX or sBTC verified via Hiro's signal-ledger event.
    Stacks(VerifiedStacksSale),
    /// Unpaid (test-only).
    None,
}

#[derive(Clone)]
pub struct SignalService {
    signal_repo: SqliteSignalRepository,
    sale_repo: SqliteSaleRepository,
    minipay_repo: SqliteMiniPayRepository,
    stacks_sale_repo: SqliteStacksSaleRepository,
    agent: SignalAgent,
    signer: ResponseSigner,
}

impl SignalService {
    pub fn new(
        signal_repo: SqliteSignalRepository,
        sale_repo: SqliteSaleRepository,
        minipay_repo: SqliteMiniPayRepository,
        stacks_sale_repo: SqliteStacksSaleRepository,
        agent: SignalAgent,
        signer: ResponseSigner,
    ) -> Self {
        Self {
            signal_repo,
            sale_repo,
            minipay_repo,
            stacks_sale_repo,
            agent,
            signer,
        }
    }

    pub async fn produce(
        &self,
        pair: &str,
        payment: SignalPayment,
    ) -> Result<SignedResponse, AppError> {
        let pair = normalize_pair(pair).map_err(AppError::BadRequest)?;
        let history = self.signal_repo.last_n_for_pair(&pair, 20).await?;
        let signal = self.agent.decide_signal(&pair, &history).await?;
        let signature = self.signer.sign(&signal.canonical_bytes());

        match payment {
            SignalPayment::X402(payment) => {
                self.sale_repo
                    .insert_signal_and_sale(&signal, &signature, &payment)
                    .await?;
            }
            SignalPayment::MiniPay {
                payment,
                block_number,
            } => {
                self.minipay_repo
                    .insert_signal_and_payment(&signal, &signature, &payment, block_number)
                    .await?;
            }
            SignalPayment::Stacks(sale) => {
                self.stacks_sale_repo
                    .insert_signal_and_sale(&signal, &signature, &sale)
                    .await?;
            }
            SignalPayment::None => {
                self.signal_repo.insert_signed(&signal, &signature).await?;
            }
        }

        Ok(SignedResponse::new(
            signal,
            &signature,
            &self.signer.public_key_bytes(),
        ))
    }
}
