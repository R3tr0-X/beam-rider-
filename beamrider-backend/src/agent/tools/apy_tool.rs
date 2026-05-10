use crate::domain::{ApyQuote, Venue};

#[derive(Debug, Clone, Default)]
pub struct ApyTool;

impl ApyTool {
    pub fn demo_quotes() -> Vec<ApyQuote> {
        vec![
            ApyQuote {
                venue: Venue::AaveCelo,
                chain_id: 42220,
                gross_apy_bps: 420,
                estimated_gas_bps: 5,
            },
            ApyQuote {
                venue: Venue::AaveArbitrum,
                chain_id: 42161,
                gross_apy_bps: 510,
                estimated_gas_bps: 25,
            },
        ]
    }
}
