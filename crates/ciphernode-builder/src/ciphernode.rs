// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::Addr;
use e3_data::{DataStore, InMemStore, StoreAddr};
use e3_events::{BusHandle, EnclaveEvent, EventBus, HistoryCollector};

#[derive(Clone, Debug)]
pub struct CiphernodeHandle {
    pub address: String,
    pub store: DataStore,
    pub bus: BusHandle,
    pub history: Option<Addr<HistoryCollector<EnclaveEvent>>>,
    pub errors: Option<Addr<HistoryCollector<EnclaveEvent>>>,
}

impl CiphernodeHandle {
    pub fn new(
        address: String,
        store: DataStore,
        bus: BusHandle,
        history: Option<Addr<HistoryCollector<EnclaveEvent>>>,
        errors: Option<Addr<HistoryCollector<EnclaveEvent>>>,
    ) -> Self {
        Self {
            address,
            store,
            bus,
            history,
            errors,
        }
    }

    pub fn bus(&self) -> &BusHandle {
        &self.bus
    }

    pub fn history(&self) -> Option<Addr<HistoryCollector<EnclaveEvent>>> {
        self.history.clone()
    }

    pub fn errors(&self) -> Option<Addr<HistoryCollector<EnclaveEvent>>> {
        self.errors.clone()
    }

    pub fn address(&self) -> String {
        self.address.clone()
    }

    pub fn store(&self) -> &DataStore {
        &self.store
    }

    pub fn in_mem_store(&self) -> Option<&Addr<InMemStore>> {
        let addr = self.store.get_addr();
        if let StoreAddr::InMem(ref store) = addr {
            return Some(store);
        };

        None
    }
}
