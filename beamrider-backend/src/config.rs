use std::time::Duration;

use crate::domain::{PaymentRequirement, PaymentResource};
use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub sqlite_max_connections: u32,
    pub request_body_limit_bytes: usize,
    pub gemini_api_key: Option<String>,
    pub gemini_model: String,
    pub ed25519_signing_key: Option<String>,
    pub x402: X402Config,
    pub session: SessionConfig,
    pub minipay: MiniPayConfig,
    pub stacks: StacksConfig,
    pub enable_workers: bool,
}

#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// Default number of signal calls each session voucher buys.
    pub default_requests: i64,
    /// How long an issued voucher stays redeemable, in seconds.
    pub ttl_seconds: i64,
}

#[derive(Debug, Clone)]
pub struct MiniPayConfig {
    /// Whether Forno verification is enabled. Tests / fixture mode keep this off.
    pub enabled: bool,
    pub forno_url: String,
    pub cusd_address: String,
    pub receiver: String,
    pub min_amount_atoms: String,
    pub min_confirmations: u64,
}

#[derive(Debug, Clone)]
pub struct StacksConfig {
    pub enabled: bool,
    pub hiro_api_url: String,
    pub signal_ledger_principal: Option<String>,
    pub signal_oracle_principal: Option<String>,
    pub relayer_principal: Option<String>,
    pub relayer_private_key: Option<String>,
    pub relay_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct X402Config {
    pub facilitator_url: String,
    pub bearer_token: Option<String>,
    pub pay_to: String,
    pub amount_atoms: String,
    pub api_base_url: String,
    pub accepted: Vec<X402NetworkConfig>,
}

#[derive(Debug, Clone)]
pub struct X402NetworkConfig {
    pub network: String,
    pub chain_id: i64,
    pub asset: String,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, AppError> {
        dotenvy::dotenv().ok();

        let settings = config::Config::builder()
            .add_source(config::Environment::default())
            .build()
            .map_err(|err| AppError::Config(err.to_string()))?;

        let host = setting_string(&settings, &["HOST", "host"], "0.0.0.0");
        let port = setting_parse(&settings, &["PORT", "port"], 8080)?;
        let database_url = setting_string(
            &settings,
            &["DATABASE_URL", "database_url"],
            "sqlite:./beamrider.db",
        );
        let sqlite_max_connections = setting_parse(
            &settings,
            &["SQLITE_MAX_CONNECTIONS", "sqlite_max_connections"],
            5,
        )?;
        let request_body_limit_bytes = setting_parse(
            &settings,
            &["REQUEST_BODY_LIMIT_BYTES", "request_body_limit_bytes"],
            64 * 1024,
        )?;
        let gemini_api_key = setting_optional(&settings, &["GEMINI_API_KEY", "gemini_api_key"]);
        let gemini_model = setting_string(
            &settings,
            &["GEMINI_MODEL", "gemini_model"],
            "gemini-2.0-flash",
        );
        let ed25519_signing_key =
            setting_optional(&settings, &["ED25519_SIGNING_KEY", "ed25519_signing_key"]);
        let enable_workers =
            setting_parse(&settings, &["ENABLE_WORKERS", "enable_workers"], false)?;

        let pay_to = setting_string(
            &settings,
            &[
                "X402_PAY_TO",
                "x402_pay_to",
                "AGENT_ADDRESS",
                "agent_address",
            ],
            "0x0000000000000000000000000000000000000000",
        );
        let amount_atoms = setting_string(
            &settings,
            &["X402_PRICE_USDC_ATOMS", "x402_price_usdc_atoms"],
            "100000",
        );
        let api_base_url = setting_string(
            &settings,
            &["API_BASE_URL", "api_base_url"],
            "http://localhost:8080",
        );
        let facilitator_url = setting_string(
            &settings,
            &["X402_FACILITATOR_URL", "x402_facilitator_url"],
            "https://api.cdp.coinbase.com/platform/v2/x402/verify",
        );
        let bearer_token = setting_optional(
            &settings,
            &[
                "X402_FACILITATOR_BEARER_TOKEN",
                "x402_facilitator_bearer_token",
                "COINBASE_CDP_API_KEY",
                "coinbase_cdp_api_key",
            ],
        );

        let session_default_requests = setting_parse(
            &settings,
            &["SESSION_DEFAULT_REQUESTS", "session_default_requests"],
            20_i64,
        )?;
        let session_ttl_seconds = setting_parse(
            &settings,
            &["SESSION_TTL_SECONDS", "session_ttl_seconds"],
            24 * 60 * 60_i64,
        )?;

        let minipay_enabled =
            setting_parse(&settings, &["MINIPAY_ENABLED", "minipay_enabled"], false)?;
        let forno_url = setting_string(
            &settings,
            &["CELO_FORNO_URL", "celo_forno_url"],
            "https://forno.celo.org",
        );
        let cusd_address = setting_string(
            &settings,
            &["CELO_CUSD_ADDRESS", "celo_cusd_address"],
            "0x765DE816845861e75A25fCA122bb6898B8B1282a",
        );
        let minipay_receiver = setting_string(
            &settings,
            &[
                "MINIPAY_RECEIVER",
                "minipay_receiver",
                "AGENT_ADDRESS",
                "agent_address",
            ],
            &pay_to,
        );
        let minipay_min_amount = setting_string(
            &settings,
            &["MINIPAY_MIN_AMOUNT_ATOMS", "minipay_min_amount_atoms"],
            "100000000000000000",
        );
        let minipay_confirmations = setting_parse(
            &settings,
            &["MINIPAY_MIN_CONFIRMATIONS", "minipay_min_confirmations"],
            1_u64,
        )?;

        let stacks_enabled =
            setting_parse(&settings, &["STACKS_ENABLED", "stacks_enabled"], false)?;
        let stacks_hiro_url = setting_string(
            &settings,
            &["STACKS_HIRO_URL", "stacks_hiro_url"],
            "https://api.hiro.so",
        );
        let stacks_signal_ledger =
            setting_optional(&settings, &["STACKS_SIGNAL_LEDGER", "stacks_signal_ledger"]);
        let stacks_signal_oracle =
            setting_optional(&settings, &["STACKS_SIGNAL_ORACLE", "stacks_signal_oracle"]);
        let stacks_relayer_principal = setting_optional(
            &settings,
            &["STACKS_RELAYER_PRINCIPAL", "stacks_relayer_principal"],
        );
        let stacks_relayer_private_key = setting_optional(
            &settings,
            &["STACKS_RELAYER_PRIVATE_KEY", "stacks_relayer_private_key"],
        );
        let stacks_relay_enabled = setting_parse(
            &settings,
            &["STACKS_RELAY_ENABLED", "stacks_relay_enabled"],
            false,
        )?;

        Ok(Self {
            host,
            port,
            database_url,
            sqlite_max_connections,
            request_body_limit_bytes,
            gemini_api_key,
            gemini_model,
            ed25519_signing_key,
            x402: X402Config {
                facilitator_url,
                bearer_token,
                pay_to,
                amount_atoms,
                api_base_url,
                accepted: default_x402_networks(),
            },
            session: SessionConfig {
                default_requests: session_default_requests,
                ttl_seconds: session_ttl_seconds,
            },
            minipay: MiniPayConfig {
                enabled: minipay_enabled,
                forno_url,
                cusd_address,
                receiver: minipay_receiver,
                min_amount_atoms: minipay_min_amount,
                min_confirmations: minipay_confirmations,
            },
            stacks: StacksConfig {
                enabled: stacks_enabled,
                hiro_api_url: stacks_hiro_url,
                signal_ledger_principal: stacks_signal_ledger,
                signal_oracle_principal: stacks_signal_oracle,
                relayer_principal: stacks_relayer_principal,
                relayer_private_key: stacks_relayer_private_key,
                relay_enabled: stacks_relay_enabled,
            },
            enable_workers,
        })
    }

    pub const fn http_timeout() -> Duration {
        Duration::from_secs(10)
    }

    pub fn for_test(database_url: impl Into<String>) -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 0,
            database_url: database_url.into(),
            sqlite_max_connections: 1,
            request_body_limit_bytes: 64 * 1024,
            gemini_api_key: None,
            gemini_model: "gemini-2.0-flash".to_string(),
            ed25519_signing_key: None,
            x402: X402Config {
                facilitator_url: "http://127.0.0.1/unused".to_string(),
                bearer_token: None,
                pay_to: "0x0000000000000000000000000000000000000001".to_string(),
                amount_atoms: "100000".to_string(),
                api_base_url: "http://localhost".to_string(),
                accepted: default_x402_networks(),
            },
            session: SessionConfig {
                default_requests: 5,
                ttl_seconds: 3600,
            },
            minipay: MiniPayConfig {
                enabled: false,
                forno_url: "http://127.0.0.1/unused".to_string(),
                cusd_address: "0x765DE816845861e75A25fCA122bb6898B8B1282a".to_string(),
                receiver: "0x0000000000000000000000000000000000000001".to_string(),
                min_amount_atoms: "1".to_string(),
                min_confirmations: 0,
            },
            stacks: StacksConfig {
                enabled: false,
                hiro_api_url: "http://127.0.0.1/unused".to_string(),
                signal_ledger_principal: None,
                signal_oracle_principal: None,
                relayer_principal: None,
                relayer_private_key: None,
                relay_enabled: false,
            },
            enable_workers: false,
        }
    }
}

impl X402Config {
    pub fn payment_requirements(&self) -> Vec<PaymentRequirement> {
        self.accepted
            .iter()
            .map(|network| {
                PaymentRequirement::usdc_exact(
                    network.network.clone(),
                    network.asset.clone(),
                    self.amount_atoms.clone(),
                    self.pay_to.clone(),
                )
            })
            .collect()
    }

    pub fn resource(&self, pair: &str) -> PaymentResource {
        PaymentResource {
            url: format!(
                "{}/v1/signals/{}",
                self.api_base_url.trim_end_matches('/'),
                pair
            ),
            description: format!("BeamRider signed market signal for {pair}"),
            mime_type: "application/json".to_string(),
        }
    }

    pub fn chain_id_for_network(&self, network: &str) -> Option<i64> {
        self.accepted
            .iter()
            .find(|candidate| candidate.network == network)
            .map(|candidate| candidate.chain_id)
    }
}

fn default_x402_networks() -> Vec<X402NetworkConfig> {
    vec![
        X402NetworkConfig {
            network: "eip155:8453".to_string(),
            chain_id: 8453,
            asset: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".to_string(),
        },
        X402NetworkConfig {
            network: "eip155:42161".to_string(),
            chain_id: 42161,
            asset: "0xaf88d065e77c8cC2239327C5EDb3A432268e5831".to_string(),
        },
    ]
}

fn setting_string(settings: &config::Config, keys: &[&str], default: &str) -> String {
    for key in keys {
        if let Ok(value) = settings.get_string(key) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return trimmed.to_string();
            }
        }
    }
    default.to_string()
}

fn setting_optional(settings: &config::Config, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Ok(value) = settings.get_string(key) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }
    None
}

fn setting_parse<T>(settings: &config::Config, keys: &[&str], default: T) -> Result<T, AppError>
where
    T: std::str::FromStr + Copy,
    T::Err: std::fmt::Display,
{
    for key in keys {
        if let Ok(value) = settings.get_string(key) {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                continue;
            }
            return trimmed
                .parse()
                .map_err(|err| AppError::Config(format!("invalid {key}: {err}")));
        }
    }
    Ok(default)
}
