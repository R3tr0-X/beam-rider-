use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Venue {
    AaveCelo,
    MoolaCelo,
    UbeswapLp,
    AaveArbitrum,
    AaveBase,
    ZestSbtc,
    BitflowSbtcStxLp,
    StackingDaoLiquid,
}

impl Venue {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::AaveCelo => "aave-celo",
            Self::MoolaCelo => "moola-celo",
            Self::UbeswapLp => "ubeswap-lp",
            Self::AaveArbitrum => "aave-arbitrum",
            Self::AaveBase => "aave-base",
            Self::ZestSbtc => "zest-sbtc",
            Self::BitflowSbtcStxLp => "bitflow-sbtc-stx-lp",
            Self::StackingDaoLiquid => "stacking-dao-liquid",
        }
    }

    pub fn parse_db(value: &str) -> Option<Self> {
        match value {
            "aave-celo" => Some(Self::AaveCelo),
            "moola-celo" => Some(Self::MoolaCelo),
            "ubeswap-lp" => Some(Self::UbeswapLp),
            "aave-arbitrum" => Some(Self::AaveArbitrum),
            "aave-base" => Some(Self::AaveBase),
            "zest-sbtc" => Some(Self::ZestSbtc),
            "bitflow-sbtc-stx-lp" => Some(Self::BitflowSbtcStxLp),
            "stacking-dao-liquid" => Some(Self::StackingDaoLiquid),
            _ => None,
        }
    }

    /// Whether this venue lives on the Stacks chain. Stacks venues are
    /// reached through `yield-vault.clar`, not through the Celo
    /// `YieldStrategy` contract.
    pub const fn is_stacks(&self) -> bool {
        matches!(
            self,
            Self::ZestSbtc | Self::BitflowSbtcStxLp | Self::StackingDaoLiquid
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApyQuote {
    pub venue: Venue,
    pub chain_id: i64,
    pub gross_apy_bps: i64,
    pub estimated_gas_bps: i64,
}

impl ApyQuote {
    pub const fn net_apy_bps(&self) -> i64 {
        self.gross_apy_bps - self.estimated_gas_bps
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RebalanceStatus {
    Proposed,
    Bridging,
    CctpBurnt,
    Completed,
    Failed,
}

impl RebalanceStatus {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Proposed => "proposed",
            Self::Bridging => "bridging",
            Self::CctpBurnt => "cctp_burnt",
            Self::Completed => "completed",
            Self::Failed => "failed",
        }
    }

    pub fn parse_db(value: &str) -> Option<Self> {
        match value {
            "proposed" => Some(Self::Proposed),
            "bridging" => Some(Self::Bridging),
            "cctp_burnt" => Some(Self::CctpBurnt),
            "completed" => Some(Self::Completed),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RebalancePlan {
    pub proposed_at: DateTime<Utc>,
    pub src_chain: i64,
    pub dest_chain: i64,
    pub amount_usdc_atoms: String,
    pub venue: Venue,
    pub expected_apy_bps: i64,
    pub status: RebalanceStatus,
}

impl RebalancePlan {
    pub fn proposed(
        src_chain: i64,
        dest_chain: i64,
        amount_usdc_atoms: impl Into<String>,
        venue: Venue,
        expected_apy_bps: i64,
    ) -> Self {
        Self {
            proposed_at: Utc::now(),
            src_chain,
            dest_chain,
            amount_usdc_atoms: amount_usdc_atoms.into(),
            venue,
            expected_apy_bps,
            status: RebalanceStatus::Proposed,
        }
    }
}
