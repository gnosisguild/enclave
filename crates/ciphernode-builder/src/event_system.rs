// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::get_enclave_event_bus;
use actix::{Actor, Addr, Recipient};
use anyhow::Result;
use e3_data::{
    CommitLogEventLog, DataStore, ForwardTo, InMemEventLog, InMemSequenceIndex, InMemStore,
    InsertBatch, SledSequenceIndex, SledStore, WriteBuffer,
};
use e3_events::hlc::Hlc;
use e3_events::{
    BusHandle, CommitSnapshot, EnclaveEvent, EventBus, EventBusConfig, EventStore, Sequencer,
};
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

pub enum EventStoreAddr {
    InMem(Addr<EventStore<InMemSequenceIndex, InMemEventLog>>),
    Persisted(Addr<EventStore<SledSequenceIndex, CommitLogEventLog>>),
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
    /// Hlc override
    hlc: OnceCell<Hlc>,
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
            hlc: OnceCell::new(),
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
            hlc: OnceCell::new(),
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
            hlc: OnceCell::new(),
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

    /// Add an injected hlc
    pub fn with_hlc(self, hlc: Hlc) -> Self {
        let _ = self.hlc.set(hlc);
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
            .get_or_try_init(|| match self.eventstore()? {
                EventStoreAddr::InMem(es) => {
                    Ok(Sequencer::new(&self.eventbus(), es, self.buffer()).start())
                }
                EventStoreAddr::Persisted(es) => {
                    Ok(Sequencer::new(&self.eventbus(), es, self.buffer()).start())
                }
            })
            .cloned()
    }

    /// Get the EventStore address
    pub fn eventstore(&self) -> Result<EventStoreAddr> {
        match &self.backend {
            EventSystemBackend::InMem(b) => {
                let addr = b
                    .eventstore
                    .get_or_init(|| {
                        EventStore::new(InMemSequenceIndex::new(), InMemEventLog::new()).start()
                    })
                    .clone();
                Ok(EventStoreAddr::InMem(addr))
            }
            EventSystemBackend::Persisted(b) => {
                let addr = b
                    .eventstore
                    .get_or_try_init(|| -> Result<_> {
                        let index = SledSequenceIndex::new(&b.sled_path, "sequence_index")?;
                        let log = CommitLogEventLog::new(&b.log_path)?;
                        Ok(EventStore::new(index, log).start())
                    })?
                    .clone();
                Ok(EventStoreAddr::Persisted(addr))
            }
        }
    }

    /// Get an instance of the Hlc
    pub fn hlc(&self) -> Result<Hlc> {
        self.hlc
            .get_or_try_init(|| Ok(Hlc::new(self.node_id)))
            .cloned()
    }

    /// Get the BusHandle
    pub fn handle(&self) -> Result<BusHandle> {
        self.handle
            .get_or_try_init(|| {
                Ok(BusHandle::new(
                    self.eventbus(),
                    self.sequencer()?,
                    self.hlc()?,
                ))
            })
            .cloned()
    }

    /// Get the DataStore
    pub fn store(&self) -> Result<DataStore> {
        let store = match &self.backend {
            EventSystemBackend::InMem(b) => {
                let addr = b
                    .store
                    .get_or_init(|| InMemStore::new(true).start())
                    .clone();
                DataStore::from(&addr)
            }
            EventSystemBackend::Persisted(b) => {
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

        let store: Option<Recipient<InsertBatch>> = match &self.backend {
            EventSystemBackend::InMem(b) => b.store.get().cloned().map(Into::into),
            EventSystemBackend::Persisted(b) => b.store.get().cloned().map(Into::into),
        };

        let Some(store) = store else {
            return;
        };

        // Now we know both are ready, so initialization will succeed
        self.wired.get_or_init(|| {
            buffer.do_send(ForwardTo::new(store));
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
    use std::time::Duration;

    use super::*;
    use actix::Actor;
    use actix::Handler;
    use actix::Message;
    use e3_data::Get;
    use e3_data::Insert;
    use e3_events::prelude::*;
    use e3_events::EnclaveEventData;
    use e3_events::TestEvent;
    use tempfile::TempDir;
    use tokio::time::sleep;

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

    #[actix::test]
    async fn test_correct_data() -> Result<()> {
        let system = EventSystem::in_mem("cn1").with_fresh_bus();
        let seqencer = system.sequencer()?;
        let handle = system.handle()?;
        let store = system.store()?;
        let buffer = system.buffer();
        let eventstore = system.eventstore()?;

        #[derive(Message, Debug)]
        #[rtype("Vec<String>")]
        struct GetLogs;

        struct Listener {
            logs: Vec<String>,
        }

        impl Handler<EnclaveEvent> for Listener {
            type Result = ();
            fn handle(&mut self, msg: EnclaveEvent, _: &mut Self::Context) -> Self::Result {
                if let EnclaveEventData::TestEvent(TestEvent { msg, .. }) = msg.into_data() {
                    self.logs.push(msg);
                }
            }
        }

        impl Handler<GetLogs> for Listener {
            type Result = Vec<String>;
            fn handle(&mut self, msg: GetLogs, _: &mut Self::Context) -> Self::Result {
                self.logs.clone()
            }
        }

        impl Actor for Listener {
            type Context = actix::Context<Self>;
        }

        buffer.do_send(Insert::new("/foo/name", b"Fred".into()));
        buffer.do_send(Insert::new("/foo/age", b"21".into()));
        buffer.do_send(Insert::new("/foo/occupation", b"developer".into()));

        let r = store.scope("name").read::<Vec<u8>>().await?;
        assert_eq!(r, None);

        let listener = Listener { logs: Vec::new() }.start();
        handle.subscribe("*", listener.clone().into());
        handle.publish(TestEvent::new("pink", 1))?;
        // handle.publish(TestEvent::new("yellow", 1))?;
        // handle.publish(TestEvent::new("red", 1))?;
        // handle.publish(TestEvent::new("white", 1))?;
        sleep(Duration::from_millis(100)).await;

        let logs = listener.send(GetLogs).await?;

        assert_eq!(logs, vec!["pink"]);
        // assert_eq!(logs, vec!["pink", "yellow", "red", "white"]);

        sleep(Duration::from_millis(100)).await;

        let r = store.scope("/foo/name").read::<Vec<u8>>().await?;
        assert_eq!(r, None);

        Ok(())
    }
}
