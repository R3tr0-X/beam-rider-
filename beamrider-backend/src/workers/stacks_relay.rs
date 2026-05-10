//! Stacks signal-oracle relay worker (scaffold).
//!
//! Posts the SHA-256 of each signal's canonical bytes to
//! `signal-oracle.clar::commit-signal` once per Stacks block.
//!
//! The broadcast path requires a Stacks transaction signer, which BeamRider
//! does not yet ship — per AGENTS.md we do not commit live transaction code
//! before tests for it exist. This worker therefore logs intent and exits
//! when the relayer credentials are missing, mirroring the existing
//! `earnings_watcher` pattern.

use tokio::task::JoinHandle;

use crate::state::AppState;

pub fn spawn(state: AppState) -> JoinHandle<()> {
    tokio::spawn(async move {
        let cfg = &state.config.stacks;
        if !cfg.relay_enabled {
            tracing::info!("stacks relay disabled by STACKS_RELAY_ENABLED=false; not running");
            return;
        }
        match (&cfg.relayer_principal, &cfg.relayer_private_key) {
            (Some(principal), Some(_)) => {
                tracing::info!(
                    %principal,
                    "stacks relay credentials present; broadcast path is deferred to a follow-up — \
                     see workers/stacks_relay.rs"
                );
            }
            _ => {
                tracing::info!(
                    "stacks relay enabled but STACKS_RELAYER_PRINCIPAL or PRIVATE_KEY missing; not running"
                );
            }
        }
    })
}
