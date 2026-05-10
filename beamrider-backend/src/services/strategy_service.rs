use crate::domain::ApyQuote;
use crate::error::AppError;

#[derive(Debug, Clone, Default)]
pub struct StrategyService;

impl StrategyService {
    pub fn choose_best(quotes: &[ApyQuote]) -> Result<ApyQuote, AppError> {
        let mut iter = quotes.iter();
        let first = iter.next().ok_or_else(|| {
            AppError::BadRequest("at least one APY quote is required".to_string())
        })?;
        let mut best = first.clone();

        for quote in iter {
            let quote_net = quote.net_apy_bps();
            let best_net = best.net_apy_bps();
            let better_net = quote_net > best_net;
            let deterministic_tie = quote_net == best_net
                && (quote.chain_id, quote.venue.as_str()) < (best.chain_id, best.venue.as_str());
            if better_net || deterministic_tie {
                best = quote.clone();
            }
        }

        Ok(best)
    }
}
