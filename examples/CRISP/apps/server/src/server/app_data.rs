use std::sync::Arc;

use enclave_sdk::indexer::SharedStore;
use tokio::sync::RwLock;

use super::{
    database::SledDB,
    repo::{CrispE3Repository, CurrentRoundRepository},
};

pub struct AppData {
    db: SharedStore<SledDB>,
}

impl AppData {
    pub fn new(db: SharedStore<SledDB>) -> Self {
        Self { db }
    }

    pub fn e3(&self, e3_id: u64) -> CrispE3Repository<SledDB> {
        CrispE3Repository::new(self.db.clone(), e3_id)
    }

    pub fn current_round(&self) -> CurrentRoundRepository<SledDB> {
        CurrentRoundRepository::new(self.db.clone())
    }
}
