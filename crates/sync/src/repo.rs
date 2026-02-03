// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_config::StoreKeys;
use e3_data::{Repositories, Repository};
use e3_events::AggregateId;

pub trait SyncRepositoryFactory {
    fn aggregate_seq(&self, aggregate_id: AggregateId) -> Repository<u64>;
    fn aggregate_block(&self, aggregate_id: AggregateId) -> Repository<u64>;
    fn aggregate_ts(&self, aggregate_id: AggregateId) -> Repository<u128>;
}

impl SyncRepositoryFactory for Repositories {
    fn aggregate_seq(&self, aggregate_id: AggregateId) -> Repository<u64> {
        Repository::new(self.store.scope(StoreKeys::aggregate_seq(aggregate_id)))
    }

    fn aggregate_block(&self, aggregate_id: AggregateId) -> Repository<u64> {
        Repository::new(self.store.scope(StoreKeys::aggregate_block(aggregate_id)))
    }

    fn aggregate_ts(&self, aggregate_id: AggregateId) -> Repository<u128> {
        Repository::new(self.store.scope(StoreKeys::aggregate_ts(aggregate_id)))
    }
}
