// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::get_enclave_event_bus;
use actix::{Actor, Addr};
use anyhow::Result;
use e3_data::{
    CommitLogEventLog, DataStore, ForwardTo, InMemEventLog, InMemSequenceIndex, InMemStore,
    SledSequenceIndex, SledStore, WriteBuffer,
};
use e3_events::hlc::Hlc;
use e3_events::{BusHandle, EnclaveEvent, EventBus, EventStore, Sequencer};
use once_cell::sync::OnceCell;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;

struct InMemBackend {
    eventstore: OnceCell<Addr<EventStore<InMemSequenceIndex, InMemEventLog>>>,
    store: OnceCell<Addr<InMemStore>>,
}

struct PersistedBackend {
    log_path: PathBuf,
    sled_path: PathBuf,
    eventstore: OnceCell<Addr<EventStore<SledSequenceIndex, CommitLogEventLog>>>,
    store: OnceCell<Addr<SledStore>>,
}

enum Backend {
    InMem(InMemBackend),
    Persisted(PersistedBackend),
}

pub struct EventSystem {
    node_id: u32,
    backend: Backend,
    buffer: OnceCell<Addr<WriteBuffer>>,
    sequencer: OnceCell<Addr<Sequencer>>,
    eventbus: OnceCell<Addr<EventBus<EnclaveEvent>>>,
    handle: OnceCell<BusHandle>,
    wired: OnceCell<()>,
}

impl EventSystem {
    pub fn in_mem(node_id: &str) -> Self {
        Self {
            node_id: EventSystem::node_id(node_id),
            backend: Backend::InMem(InMemBackend {
                eventstore: OnceCell::new(),
                store: OnceCell::new(),
            }),
            buffer: OnceCell::new(),
            sequencer: OnceCell::new(),
            eventbus: OnceCell::new(),
            handle: OnceCell::new(),
            wired: OnceCell::new(),
        }
    }

    pub fn persisted(node_id: &str, log_path: PathBuf, sled_path: PathBuf) -> Self {
        Self {
            node_id: EventSystem::node_id(node_id),
            backend: Backend::Persisted(PersistedBackend {
                log_path,
                sled_path,
                eventstore: OnceCell::new(),
                store: OnceCell::new(),
            }),
            buffer: OnceCell::new(),
            sequencer: OnceCell::new(),
            eventbus: OnceCell::new(),
            handle: OnceCell::new(),
            wired: OnceCell::new(),
        }
    }

    pub fn with_event_bus(self, bus: Addr<EventBus<EnclaveEvent>>) -> Self {
        let _ = self.eventbus.set(bus);
        self
    }

    pub fn eventbus(&self) -> Addr<EventBus<EnclaveEvent>> {
        self.eventbus.get_or_init(get_enclave_event_bus).clone()
    }

    pub fn buffer(&self) -> Addr<WriteBuffer> {
        let buffer = self
            .buffer
            .get_or_init(|| WriteBuffer::new().start())
            .clone();
        self.wire_if_ready();
        buffer
    }

    pub fn sequencer(&self) -> Result<Addr<Sequencer>> {
        self.sequencer
            .get_or_try_init(|| match &self.backend {
                Backend::InMem(b) => {
                    let eventstore = b
                        .eventstore
                        .get_or_init(|| {
                            EventStore::new(InMemSequenceIndex::new(), InMemEventLog::new()).start()
                        })
                        .clone();
                    Ok(Sequencer::new(&self.eventbus(), eventstore, self.buffer()).start())
                }
                Backend::Persisted(b) => {
                    let eventstore = b
                        .eventstore
                        .get_or_try_init(|| -> Result<_> {
                            let index = SledSequenceIndex::new(&b.sled_path, "sequence_index")?;
                            let log = CommitLogEventLog::new(&b.log_path)?;
                            Ok(EventStore::new(index, log).start())
                        })?
                        .clone();
                    Ok(Sequencer::new(&self.eventbus(), eventstore, self.buffer()).start())
                }
            })
            .cloned()
    }

    pub fn handle(&self) -> Result<BusHandle> {
        self.handle
            .get_or_try_init(|| {
                Ok(BusHandle::new(
                    self.eventbus(),
                    self.sequencer()?,
                    Hlc::new(self.node_id),
                ))
            })
            .cloned()
    }

    pub fn store(&self) -> Result<DataStore> {
        let store = match &self.backend {
            Backend::InMem(b) => {
                let addr = b
                    .store
                    .get_or_init(|| InMemStore::new(true).start())
                    .clone();
                DataStore::from(&addr)
            }
            Backend::Persisted(b) => {
                let addr = b
                    .store
                    .get_or_try_init(|| {
                        let handle = self.handle()?;
                        SledStore::new(&handle, &b.sled_path)
                    })?
                    .clone();
                DataStore::from(&addr)
            }
        };
        self.wire_if_ready();
        Ok(store)
    }

    fn wire_if_ready(&self) {
        let buffer = match self.buffer.get() {
            Some(b) => b,
            None => return,
        };

        self.wired.get_or_init(|| match &self.backend {
            Backend::InMem(b) => {
                if let Some(store) = b.store.get() {
                    buffer.do_send(ForwardTo::new(store.clone()));
                }
            }
            Backend::Persisted(b) => {
                if let Some(store) = b.store.get() {
                    buffer.do_send(ForwardTo::new(store.clone()));
                }
            }
        });
    }

    fn node_id(name: &str) -> u32 {
        let mut hasher = DefaultHasher::new();
        name.hash(&mut hasher);
        hasher.finish() as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[actix::test]
    async fn test_persisted() {
        let tmp = TempDir::new().unwrap();
        let system = EventSystem::persisted("cn2", tmp.path().join("log"), tmp.path().join("sled"));

        let _handle = system.handle().expect("Failed to get handle");
        system.store().expect("Failed to get store");

        // Wiring happened automatically
        assert!(system.wired.get().is_some());
    }

    #[actix::test]
    async fn test_in_mem() {
        let eventbus = EventBus::<EnclaveEvent>::default().start();
        let system = EventSystem::in_mem("cn1").with_event_bus(eventbus);

        let _handle = system.handle().expect("Failed to get handle");
        system.store().expect("Failed to get store");

        // Wiring happened automatically
        assert!(system.wired.get().is_some());
    }
}
