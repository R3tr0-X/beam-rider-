pub mod arbitrum;
pub mod base;
pub mod cctp;
pub mod celo;
pub mod stacks;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainId {
    Celo = 42220,
    Base = 8453,
    Arbitrum = 42161,
}

impl ChainId {
    pub const fn as_i64(self) -> i64 {
        self as i64
    }
}

/// Synthetic chain id used for Stacks rows in tables that share an EVM
/// `chain_id` column. Stacks has no EIP-155 number; -1 is reserved here.
pub const STACKS_CHAIN_ID: i64 = -1;
