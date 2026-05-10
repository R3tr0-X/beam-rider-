use crate::error::AppError;
use crate::repositories::{SqliteSignalRepository, StoredSignal};

#[derive(Debug, Clone)]
pub struct HistoryTool {
    repo: SqliteSignalRepository,
}

impl HistoryTool {
    pub const fn new(repo: SqliteSignalRepository) -> Self {
        Self { repo }
    }

    pub async fn recent(&self, pair: &str, limit: i64) -> Result<Vec<StoredSignal>, AppError> {
        self.repo.last_n_for_pair(pair, limit).await
    }
}
