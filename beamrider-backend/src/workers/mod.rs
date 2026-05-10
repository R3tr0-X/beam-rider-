pub mod attestation_poller;
pub mod earnings_watcher;
pub mod rebalance_scheduler;
pub mod stacks_relay;

use tokio::task::JoinHandle;

use crate::state::AppState;

pub fn spawn_enabled(state: &AppState) -> Vec<JoinHandle<()>> {
    if !state.config.enable_workers {
        return Vec::new();
    }

    vec![
        earnings_watcher::spawn(state.clone()),
        rebalance_scheduler::spawn(state.clone()),
        attestation_poller::spawn(state.clone()),
        stacks_relay::spawn(state.clone()),
    ]
}
