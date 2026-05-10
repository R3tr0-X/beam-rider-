use tokio::task::JoinHandle;

use crate::state::AppState;

pub fn spawn(_state: AppState) -> JoinHandle<()> {
    tokio::spawn(async {
        tracing::info!("earnings watcher enabled; live chain subscription is deferred in MVP");
    })
}
