use sqlx::SqlitePool;

use crate::agent::{GeminiAgentConfig, SignalAgent};
use crate::chains::stacks::StacksClient;
use crate::config::AppConfig;
use crate::crypto::ResponseSigner;
use crate::db;
use crate::error::AppError;
use crate::middleware::x402::X402Verifier;
use crate::repositories::{
    SqliteEventRepository, SqliteMiniPayRepository, SqliteSaleRepository, SqliteSessionRepository,
    SqliteSignalRepository, SqliteStacksSaleRepository, SqliteStrategyRepository,
};
use crate::services::{
    CeloPaymentVerifier, RebalanceService, SessionService, SignalService, StacksPaymentVerifier,
};

#[derive(Clone)]
pub struct AppState {
    pub config: AppConfig,
    pub pool: SqlitePool,
    pub signal_repo: SqliteSignalRepository,
    pub sale_repo: SqliteSaleRepository,
    pub minipay_repo: SqliteMiniPayRepository,
    pub stacks_sale_repo: SqliteStacksSaleRepository,
    pub event_repo: SqliteEventRepository,
    pub strategy_repo: SqliteStrategyRepository,
    pub session_repo: SqliteSessionRepository,
    pub signer: ResponseSigner,
    pub signal_service: SignalService,
    pub session_service: SessionService,
    pub rebalance_service: RebalanceService,
    pub x402: X402Verifier,
    pub celo_payment: CeloPaymentVerifier,
    pub stacks_payment: StacksPaymentVerifier,
}

impl AppState {
    pub async fn from_config(config: AppConfig) -> Result<Self, AppError> {
        let pool = db::connect(&config.database_url, config.sqlite_max_connections).await?;
        Self::from_pool(config, pool, false)
    }

    pub fn from_pool(
        config: AppConfig,
        pool: SqlitePool,
        fixture_x402: bool,
    ) -> Result<Self, AppError> {
        let client = reqwest::Client::builder()
            .timeout(AppConfig::http_timeout())
            .build()?;
        let signer = ResponseSigner::from_optional_secret(config.ed25519_signing_key.as_deref())?;
        let signal_repo = SqliteSignalRepository::new(pool.clone());
        let sale_repo = SqliteSaleRepository::new(pool.clone());
        let minipay_repo = SqliteMiniPayRepository::new(pool.clone());
        let stacks_sale_repo = SqliteStacksSaleRepository::new(pool.clone());
        let event_repo = SqliteEventRepository::new(pool.clone());
        let strategy_repo = SqliteStrategyRepository::new(pool.clone());
        let session_repo = SqliteSessionRepository::new(pool.clone());
        let gemini = config
            .gemini_api_key
            .as_ref()
            .map(|api_key| GeminiAgentConfig {
                api_key: api_key.clone(),
                model: config.gemini_model.clone(),
            });
        let agent = SignalAgent::new(gemini, client.clone());
        let signal_service = SignalService::new(
            signal_repo.clone(),
            sale_repo.clone(),
            minipay_repo.clone(),
            stacks_sale_repo.clone(),
            agent,
            signer.clone(),
        );
        let session_service = SessionService::new(config.session.clone(), session_repo.clone());
        let rebalance_service = RebalanceService::new(event_repo.clone());
        let x402 = if fixture_x402 {
            X402Verifier::fixture(config.x402.clone(), client.clone())
        } else {
            X402Verifier::cdp(config.x402.clone(), client.clone())
        };
        let celo_payment = CeloPaymentVerifier::new(config.minipay.clone(), client.clone());
        let stacks_client = StacksClient::from_config(&config.stacks, client.clone())?;
        let stacks_payment = StacksPaymentVerifier::new(&config.stacks, stacks_client);

        Ok(Self {
            config,
            pool,
            signal_repo,
            sale_repo,
            minipay_repo,
            stacks_sale_repo,
            event_repo,
            strategy_repo,
            session_repo,
            signer,
            signal_service,
            session_service,
            rebalance_service,
            x402,
            celo_payment,
            stacks_payment,
        })
    }

    pub async fn for_test() -> Result<Self, AppError> {
        let config = AppConfig::for_test("sqlite::memory:");
        let pool = db::connect_memory().await?;
        Self::from_pool(config, pool, true)
    }
}
