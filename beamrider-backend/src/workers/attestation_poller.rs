use tokio::task::JoinHandle;

use crate::state::AppState;

pub fn spawn(_state: AppState) -> JoinHandle<()> {
    tokio::spawn(async {
        tracing::info!("attestation poller enabled; Iris polling is deferred in MVP");
    })
}
