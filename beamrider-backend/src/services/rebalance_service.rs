use crate::chains::cctp::{HookAction, HookData, encode_hook_data};
use crate::domain::{ApyQuote, RebalancePlan};
use crate::error::AppError;
use crate::repositories::SqliteEventRepository;
use crate::services::strategy_service::StrategyService;

#[derive(Debug, Clone)]
pub struct PlannedRebalance {
    pub id: i64,
    pub plan: RebalancePlan,
    pub hook_data: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct RebalanceService {
    event_repo: SqliteEventRepository,
}

impl RebalanceService {
    pub fn new(event_repo: SqliteEventRepository) -> Self {
        Self { event_repo }
    }

    pub async fn plan_best(
        &self,
        src_chain: i64,
        amount_usdc_atoms: &str,
        quotes: &[ApyQuote],
        destination_vault: &str,
    ) -> Result<PlannedRebalance, AppError> {
        let best = StrategyService::choose_best(quotes)?;
        let expected_apy_bps = best.net_apy_bps();
        let plan = RebalancePlan::proposed(
            src_chain,
            best.chain_id,
            amount_usdc_atoms,
            best.venue,
            expected_apy_bps,
        );
        let hook_data = encode_hook_data(&HookData {
            action: HookAction::DepositAave,
            destination_vault: destination_vault.to_string(),
            metadata: plan.venue.as_str().as_bytes().to_vec(),
        })?;
        let id = self.event_repo.insert_rebalance(&plan).await?;

        Ok(PlannedRebalance {
            id,
            plan,
            hook_data,
        })
    }
}
