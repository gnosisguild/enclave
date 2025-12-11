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
use e3_events::{BusHandle, EnclaveEvent, EventBus, EventBusConfig, EventStore, Sequencer};
use once_cell::sync::OnceCell;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;

/// Hold the InMem EventStore instance and InMemStore
struct InMemBackend {
    eventstore: OnceCell<Addr<EventStore<InMemSequenceIndex, InMemEventLog>>>,
    store: OnceCell<Addr<InMemStore>>,
}

/// Hold the Persistent EventStore instance and SledStore
struct PersistedBackend {
    log_path: PathBuf,
    sled_path: PathBuf,
    eventstore: OnceCell<Addr<EventStore<SledSequenceIndex, CommitLogEventLog>>>,
    store: OnceCell<Addr<SledStore>>,
}

/// An EventSystemBackend is holding the potentially persistent structures for the system
enum EventSystemBackend {
    InMem(InMemBackend),
    Persisted(PersistedBackend),
}

/// EventSystem holds interconnected references to the components that manage events and
/// persistence within the node. The EventSystem connects:
///
/// - **BusHandle** for interacting with the event system
/// - **EventBus** for managing publishing of events to listeners
/// - **EventStore** for managing persistence of events
/// - **Sequencer** for managing sequencing of event persistence and snapshot coordination
/// - **WriteBuffer** for batching inserts from actors into a snapshot
///
pub struct EventSystem {
    /// A nodes id to be used as a tiebreaker in logical clock timestamp differentiation
    node_id: u32,
    /// EventSystem backend either persisted or in memory
    backend: EventSystemBackend,
    /// WriteBuffer for batching inserts from actors into a snapshot
    buffer: OnceCell<Addr<WriteBuffer>>,
    /// EventSystem Sequencer
    sequencer: OnceCell<Addr<Sequencer>>,
    /// EventSystem eventbus
    eventbus: OnceCell<Addr<EventBus<EnclaveEvent>>>,
    /// EventSystem BusHandle
    handle: OnceCell<BusHandle>,
    /// A OnceLock that is used to indicate whether the system is wired to write snapshots
    wired: OnceCell<()>,
}

impl EventSystem {
    /// Create a new in memory EventSystem with default settings
    pub fn new(name: &str) -> Self {
        EventSystem::in_mem(name)
    }

    /// Create an in memory EventSystem
    pub fn in_mem(node_id: &str) -> Self {
        Self {
            node_id: EventSystem::node_id(node_id),
            backend: EventSystemBackend::InMem(InMemBackend {
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

    /// Create an in memory EventSystem with a given store
    pub fn in_mem_from_store(node_id: &str, store: &Addr<InMemStore>) -> Self {
        Self {
            node_id: EventSystem::node_id(node_id),
            backend: EventSystemBackend::InMem(InMemBackend {
                eventstore: OnceCell::new(),
                store: OnceCell::from(store.to_owned()),
            }),
            buffer: OnceCell::new(),
            sequencer: OnceCell::new(),
            eventbus: OnceCell::new(),
            handle: OnceCell::new(),
            wired: OnceCell::new(),
        }
    }

    /// Create a persisted EventSystem with datafiles at the given paths
    pub fn persisted(node_id: &str, log_path: PathBuf, sled_path: PathBuf) -> Self {
        Self {
            node_id: EventSystem::node_id(node_id),
            backend: EventSystemBackend::Persisted(PersistedBackend {
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

    /// Pass in a sepecific given event bus
    pub fn with_event_bus(self, bus: Addr<EventBus<EnclaveEvent>>) -> Self {
        let _ = self.eventbus.set(bus);
        self
    }

    /// Use a fresh event bus that is not the default singleton instance
    pub fn with_fresh_bus(self) -> Self {
        let _ = self
            .eventbus
            .set(EventBus::new(EventBusConfig { deduplicate: true }).start());
        self
    }

    /// Get the eventbus address
    pub fn eventbus(&self) -> Addr<EventBus<EnclaveEvent>> {
        self.eventbus.get_or_init(get_enclave_event_bus).clone()
    }

    /// Get the buffer address
    pub fn buffer(&self) -> Addr<WriteBuffer> {
        let buffer = self
            .buffer
            .get_or_init(|| WriteBuffer::new().start())
            .clone();
        self.wire_if_ready();
        buffer
    }

    /// Get the sequencer address
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

    /// Get the BusHandle
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

    /// Get the DataStore
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

    // We need to ensure that once the buffer and store are created they are connected so that
    // inserts are sent between the two actors. This internal function ensures this happens.
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
