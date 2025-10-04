use actix::Addr;
use e3_data::InMemStore;
use e3_events::{EnclaveEvent, ErrorCollector, EventBus, HistoryCollector};

#[derive(Clone, Debug)]
pub struct CiphernodeSimulated {
    pub address: String,
    pub store: Addr<InMemStore>,
    pub bus: Addr<EventBus<EnclaveEvent>>,
    pub history: Option<Addr<HistoryCollector<EnclaveEvent>>>,
    pub errors: Option<Addr<ErrorCollector<EnclaveEvent>>>,
}

impl CiphernodeSimulated {
    pub fn new(
        address: String,
        store: Addr<InMemStore>,
        bus: Addr<EventBus<EnclaveEvent>>,
        history: Option<Addr<HistoryCollector<EnclaveEvent>>>,
        errors: Option<Addr<ErrorCollector<EnclaveEvent>>>,
    ) -> Self {
        Self {
            address,
            store,
            bus,
            history,
            errors,
        }
    }

    pub fn bus(&self) -> Addr<EventBus<EnclaveEvent>> {
        self.bus.clone()
    }

    pub fn history(&self) -> Option<Addr<HistoryCollector<EnclaveEvent>>> {
        self.history.clone()
    }

    pub fn errors(&self) -> Option<Addr<ErrorCollector<EnclaveEvent>>> {
        self.errors.clone()
    }

    pub fn address(&self) -> String {
        self.address.clone()
    }

    pub fn store(&self) -> Addr<InMemStore> {
        self.store.clone()
    }
}
