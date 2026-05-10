use tokio::task::JoinHandle;

use crate::state::AppState;

pub fn spawn(_state: AppState) -> JoinHandle<()> {
    tokio::spawn(async {
        tracing::info!("rebalance scheduler enabled; live execution is deferred in MVP");
    })
}
