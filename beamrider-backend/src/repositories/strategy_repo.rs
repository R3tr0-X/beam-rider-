use sqlx::SqlitePool;

use crate::error::AppError;
use crate::repositories::event_repo::{SqliteEventRepository, StoredRebalance};

#[derive(Debug, Clone)]
pub struct SqliteStrategyRepository {
    events: SqliteEventRepository,
}

impl SqliteStrategyRepository {
    pub const fn new(pool: SqlitePool) -> Self {
        Self {
            events: SqliteEventRepository::new(pool),
        }
    }

    pub async fn current_position(&self) -> Result<Option<StoredRebalance>, AppError> {
        self.events.latest_rebalance().await
    }
}
