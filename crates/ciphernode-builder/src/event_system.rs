// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::get_enclave_event_bus;
use actix::{Actor, Addr, Recipient};
use anyhow::{anyhow, Result};
use e3_data::{
    CommitLogEventLog, DataStore, ForwardTo, InMemEventLog, InMemSequenceIndex, InMemStore,
    InsertBatch, SledSequenceIndex, SledStore, WriteBuffer,
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

pub enum EventStoreAddr {
    InMem(Addr<EventStore<InMemSequenceIndex, InMemEventLog>>),
    Persisted(Addr<EventStore<SledSequenceIndex, CommitLogEventLog>>),
}

impl TryFrom<EventStoreAddr> for Addr<EventStore<InMemSequenceIndex, InMemEventLog>> {
    type Error = anyhow::Error;
    fn try_from(value: EventStoreAddr) -> std::result::Result<Self, Self::Error> {
        if let EventStoreAddr::InMem(addr) = value {
            Ok(addr)
        } else {
            Err(anyhow!(
                "address was not EventStore<InMemSequenceIndex, InMemEventLog>"
            ))
        }
    }
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
    /// Creates an in-memory EventSystem configured from the provided name.
    ///
    /// # Examples
    ///
    /// ```
    /// let sys = EventSystem::new("node1");
    /// // `sys` is ready to be used with its in-memory backend.
    /// ```
    pub fn new(name: &str) -> Self {
        EventSystem::in_mem(name)
    }

    /// Create an EventSystem configured to use an in-memory backend.
    ///
    /// The `node_id` string is hashed to derive the internal `u32` node identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// let sys = EventSystem::in_mem("local-node");
    /// // `sys` uses in-memory stores and lazy-initializes actors when accessed.
    /// ```
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

    /// Constructs an in-memory EventSystem that uses the provided `InMemStore`.
    ///
    /// The `node_id` string is used to derive the system's numeric node identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assume `store` is an existing Addr<InMemStore> obtained elsewhere.
    /// let system = EventSystem::in_mem_from_store("node-a", &store);
    /// ```
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

    /// Construct an EventSystem configured to use persisted storage at the specified file paths.
    ///
    /// The `node_id` string is hashed to derive the internal node identifier. `log_path` is the
    /// filesystem path for the commit log, and `sled_path` is the path for the Sled key-value store.
    ///
    /// # Arguments
    ///
    /// * `node_id` - A human-readable identifier used to derive an internal u32 node identifier.
    /// * `log_path` - Path to the commit log file to be used by the persisted event log.
    /// * `sled_path` - Path to the directory used by the Sled database.
    ///
    /// # Returns
    ///
    /// An EventSystem instance configured to use persisted backends (CommitLog + Sled).
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// let sys = EventSystem::persisted("node-a", PathBuf::from("commits.log"), PathBuf::from("sled_db"));
    /// ```
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

    /// Sets the EventBus address to be used by this EventSystem.
    ///
    /// This overrides any previously configured bus for the instance.
    ///
    /// # Returns
    ///
    /// The EventSystem with the provided EventBus configured.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assume `bus_addr` is an `Addr<EventBus<EnclaveEvent>>` obtained elsewhere.
    /// let sys = EventSystem::in_mem("node").with_event_bus(bus_addr);
    /// ```
    pub fn with_event_bus(self, bus: Addr<EventBus<EnclaveEvent>>) -> Self {
        let _ = self.eventbus.set(bus);
        self
    }

    /// Replaces the system's EventBus with a newly created, deduplicating EventBus.
    ///
    /// Returns the modified EventSystem with its event bus set to a fresh instance.
    ///
    /// # Examples
    ///
    /// ```
    /// let sys = EventSystem::in_mem("node").with_fresh_bus();
    /// // `sys` now uses a fresh EventBus with deduplication enabled
    /// ```
    pub fn with_fresh_bus(self) -> Self {
        let _ = self
            .eventbus
            .set(EventBus::new(EventBusConfig { deduplicate: true }).start());
        self
    }

    /// Injects a high-level clock (Hlc) to be used by the EventSystem.
    ///
    /// This sets the HLC instance that the event system will use for generating timestamps
    /// and returns the moved `EventSystem` to allow builder-style chaining.
    ///
    /// # Examples
    ///
    /// ```
    /// let sys = EventSystem::in_mem("node").with_hlc(Hlc::default());
    /// ```
    pub fn with_hlc(self, hlc: Hlc) -> Self {
        let _ = self.hlc.set(hlc);
        self
    }

    /// Obtain the EventBus actor address used by the system.
    ///
    /// This will lazily initialize and return the shared EventBus if one is not already configured.
    ///
    /// # Examples
    ///
    /// ```
    /// let system = EventSystem::new("node");
    /// let bus = system.eventbus();
    /// // `bus` is an Addr<EventBus<EnclaveEvent>>
    /// ```
    pub fn eventbus(&self) -> Addr<EventBus<EnclaveEvent>> {
        self.eventbus.get_or_init(get_enclave_event_bus).clone()
    }

    /// Returns the system's WriteBuffer actor address, creating the buffer if it does not yet exist and initiating wiring with other components when possible.
    ///
    /// # Examples
    ///
    /// ```
    /// let sys = EventSystem::new("node");
    /// let _buffer_addr = sys.buffer();
    /// ```
    pub fn buffer(&self) -> Addr<WriteBuffer> {
        let buffer = self
            .buffer
            .get_or_init(|| WriteBuffer::new().start())
            .clone();
        self.wire_if_ready();
        buffer
    }

    /// Obtain the Sequencer actor address, initializing and starting it if it has not been created yet.
    ///
    /// The Sequencer is created using the system's EventBus, EventStore, and WriteBuffer when first requested.
    ///
    /// # Returns
    ///
    /// The address of the Sequencer actor.
    ///
    /// # Examples
    ///
    /// ```
    /// let system = EventSystem::new("node");
    /// let _sequencer = system.sequencer().unwrap();
    /// ```
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

    /// Retrieve the address of the configured EventStore, initializing it if necessary.
    ///
    /// Initializes and returns an in-memory EventStore for the in-memory backend or initializes
    /// the sled sequence index and commit log and returns a persisted EventStore for the persisted backend.
    ///
    /// # Errors
    ///
    /// Returns an error if persisted backend initialization (sled index or commit log) fails.
    ///
    /// # Examples
    ///
    /// ```
    /// let sys = EventSystem::in_mem("node");
    /// let store_addr = sys.eventstore().unwrap();
    /// match store_addr {
    ///     EventStoreAddr::InMem(_) => {},
    ///     EventStoreAddr::Persisted(_) => panic!("expected in-memory backend"),
    /// }
    /// ```
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

    /// Provides the system's high-level clock, initializing it with the system node id if it has not been created yet.
    ///
    /// The returned `Hlc` is a clone of the internally stored clock instance.
    ///
    /// # Examples
    ///
    /// ```
    /// let sys = EventSystem::in_mem("node-1");
    /// let _hlc = sys.hlc().unwrap();
    /// ```
    pub fn hlc(&self) -> Result<Hlc> {
        self.hlc
            .get_or_try_init(|| Ok(Hlc::new(self.node_id)))
            .cloned()
    }

    /// Returns a BusHandle connected to this EventSystem, initializing it lazily if needed.
    ///
    /// The returned handle coordinates the eventbus, sequencer, and HLC for publishing and querying events.
    ///
    /// # Errors
    ///
    /// Returns an `Err` if initialization of the sequencer or HLC fails.
    ///
    /// # Examples
    ///
    /// ```
    /// let system = EventSystem::new("node");
    /// let handle = system.handle().expect("failed to create BusHandle");
    /// ```
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

    /// Obtain a DataStore view backed by the event system's configured backend.
    ///
    /// This returns a DataStore that routes write operations through the system's
    /// WriteBuffer and is backed by either an in-memory store or a sled-backed store
    /// depending on the EventSystem backend configuration. If the underlying store
    /// has not yet been created it will be initialized lazily. Wiring between the
    /// buffer and the store is attempted before returning.
    ///
    /// # Errors
    ///
    /// Returns an error if initializing or accessing the persisted store fails (for
    /// the persisted backend).
    ///
    /// # Examples
    ///
    /// ```
    /// # use actix::System;
    /// # use ciphernode_builder::event_system::EventSystem;
    /// let sys = System::new();
    /// sys.block_on(async {
    ///     let es = EventSystem::new("node");
    ///     let ds = es.store().unwrap();
    ///     // use `ds` for reads/writes...
    /// });
    /// ```
    pub fn store(&self) -> Result<DataStore> {
        let store = match &self.backend {
            EventSystemBackend::InMem(b) => {
                let addr = b
                    .store
                    .get_or_init(|| InMemStore::new(true).start())
                    .clone();
                DataStore::from_in_mem(&addr, &self.buffer())
            }
            EventSystemBackend::Persisted(b) => {
                let addr = b
                    .store
                    .get_or_try_init(|| {
                        let handle = self.handle()?;
                        SledStore::new(&handle, &b.sled_path)
                    })?
                    .clone();
                DataStore::from_sled_store(&addr, &self.buffer())
            }
        };
        self.wire_if_ready();
        Ok(store)
    }

    // We need to ensure that once the buffer and store are created they are connected so that
    // inserts are sent between the two actors. This internal function ensures this happens.
    /// Ensures the write buffer is forwarded to the underlying store when both are initialized.
    ///
    /// When both the system's WriteBuffer and a store `Recipient<InsertBatch>` are available, instructs the buffer to forward batches to that store. The operation is idempotent: subsequent calls do nothing once wiring has occurred.
    ///
    /// # Examples
    ///
    /// ```
    /// // Given an initialized `EventSystem` named `es`:
    /// es.wire_if_ready();
    /// ```
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

    /// Derives a deterministic 32-bit node identifier from a name string.
    ///
    /// The result is a u32 value computed by hashing `name`.
    ///
    /// # Examples
    ///
    /// ```
    /// let a = node_id("alice");
    /// let b = node_id("alice");
    /// assert_eq!(a, b);
    /// ```
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

    use e3_events::prelude::*;
    use e3_events::EnclaveEventData;
    use e3_events::GetEventsAfter;
    use e3_events::ReceiveEvents;
    use e3_events::TestEvent;
    use tempfile::TempDir;
    use tokio::time::sleep;

    // Setup Listener for the test
    #[derive(Message, Debug)]
    #[rtype("Vec<String>")]
    struct GetLogs;

    #[derive(Message, Debug)]
    #[rtype("Vec<String>")]
    struct GetEvents;

    struct Listener {
        logs: Vec<String>,
        events: Vec<EnclaveEvent>,
    }

    impl Handler<EnclaveEvent> for Listener {
        type Result = ();
        /// Handles an incoming `EnclaveEvent` and records the contained `TestEvent` message.
        ///
        /// When the event's payload is a `TestEvent`, its `msg` field is appended to the listener's
        /// `logs` collection; other event types are ignored.
        ///
        /// # Examples
        ///
        /// ```
        /// # use ciphernode_builder::event_system::{Listener, EnclaveEvent, EnclaveEventData, TestEvent};
        /// # use actix::Context;
        /// let mut listener = Listener { logs: Vec::new(), events: Vec::new() };
        /// // construct a TestEvent-wrapped EnclaveEvent (details depend on crate constructors)
        /// let test_ev = EnclaveEvent::from(EnclaveEventData::TestEvent(TestEvent { msg: "hello".into(), ts: 0 }));
        /// // call the handler directly (context parameter is not used)
        /// listener.handle(test_ev, &mut Context::from_waker(std::task::noop_waker_ref()));
        /// assert_eq!(listener.logs.last().map(String::as_str), Some("hello"));
        /// ```
        fn handle(&mut self, msg: EnclaveEvent, _: &mut Self::Context) -> Self::Result {
            if let EnclaveEventData::TestEvent(TestEvent { msg, .. }) = msg.into_data() {
                self.logs.push(msg);
            }
        }
    }

    impl Handler<GetLogs> for Listener {
        type Result = Vec<String>;
        /// Returns a clone of the listener's stored logs.
        ///
        /// # Examples
        ///
        /// ```
        /// // Construct a listener and retrieve its logs.
        /// let mut listener = Listener { logs: vec!["entry".to_string()], events: vec![] };
        /// let logs = listener.logs.clone();
        /// assert_eq!(logs, vec!["entry".to_string()]);
        /// ```
        fn handle(&mut self, _: GetLogs, _: &mut Self::Context) -> Self::Result {
            self.logs.clone()
        }
    }

    impl Handler<GetEvents> for Listener {
        type Result = Vec<String>;
        /// Collects the `msg` fields from any `TestEvent` entries in the listener's stored events.
        ///
        /// # Examples
        ///
        /// ```
        /// // assume `listener` is a mutable Listener with some EnclaveEvent entries,
        /// // and `ctx` is a mutable actor context available in the test.
        /// let msgs: Vec<String> = listener.handle(GetEvents, &mut ctx);
        /// // `msgs` now contains the `msg` of each `TestEvent` in insertion order.
        /// ```
        fn handle(&mut self, _: GetEvents, _: &mut Self::Context) -> Self::Result {
            self.events
                .iter()
                .filter_map(|event| {
                    if let EnclaveEventData::TestEvent(evt) = event.get_data() {
                        return Some(evt.msg.clone());
                    }
                    None
                })
                .collect::<Vec<_>>()
        }
    }

    impl Handler<ReceiveEvents> for Listener {
        type Result = ();
        /// Replace the listener's stored events with the events carried by the incoming message.
        ///
        /// # Parameters
        ///
        /// - `msg`: message containing the sequence of `EnclaveEvent`s to store in the listener.
        ///
        /// The handler assigns the message's events to the actor's `events` field.
        fn handle(&mut self, msg: ReceiveEvents, _: &mut Self::Context) -> Self::Result {
            self.events = msg.events().clone();
        }
    }

    impl Actor for Listener {
        type Context = actix::Context<Self>;
    }

    /// Verifies that a persisted EventSystem initializes its components and automatically wires the buffer to the store.
    ///
    /// Creates a persisted EventSystem, obtains its BusHandle and DataStore, and asserts that wiring has occurred.
    ///
    /// # Examples
    ///
    /// ```
    /// #[actix::test]
    /// async fn test_persisted() {
    ///     let tmp = tempfile::tempdir().unwrap();
    ///     let system = EventSystem::persisted("cn2", tmp.path().join("log"), tmp.path().join("sled"));
    ///
    ///     let _handle = system.handle().expect("Failed to get handle");
    ///     system.store().expect("Failed to get store");
    ///
    ///     // Wiring happened automatically
    ///     assert!(system.wired.get().is_some());
    /// }
    /// ```
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

    /// Integration test that verifies in-memory EventSystem wiring, eventual consistency, event publishing, and event retrieval.
    ///
    /// This test sets up an in-memory EventSystem with a fresh EventBus, attaches a listener actor,
    /// writes values to the DataStore (which are only persisted after the corresponding event is published),
    /// publishes events, asserts the listener receives the events in order, and queries the in-memory EventStore
    /// for events after a captured timestamp.
    ///
    /// # Examples
    ///
    /// ```
    /// # async fn run() -> anyhow::Result<()> {
    /// let system = EventSystem::in_mem("cn1").with_fresh_bus();
    /// let handle = system.handle()?;
    /// let datastore = system.store()?;
    /// let listener = Listener { logs: Vec::new(), events: Vec::new() }.start();
    /// handle.subscribe("*", listener.clone().into());
    /// datastore.scope("/a").write("v".to_string());
    /// assert_eq!(datastore.scope("/a").read::<String>().await?, None);
    /// handle.publish(TestEvent::new("e", 1))?;
    /// tokio::time::sleep(std::time::Duration::from_millis(1)).await;
    /// assert_eq!(datastore.scope("/a").read::<String>().await?, Some("v".to_string()));
    /// # Ok(()) }
    /// ```
    #[actix::test]
    async fn test_event_system() -> Result<()> {
        let system = EventSystem::in_mem("cn1").with_fresh_bus();
        let handle = system.handle()?;
        let datastore = system.store()?;
        let eventstore = system.eventstore()?;
        let listener = Listener {
            logs: Vec::new(),
            events: Vec::new(),
        }
        .start();

        // Send all evts to the listener
        handle.subscribe("*", listener.clone().into());

        // Lets store some data
        datastore.scope("/foo/name").write("Fred".to_string());
        datastore.scope("/foo/age").write(21u64);
        datastore
            .scope("/foo/occupation")
            .write("developer".to_string());

        // NOTE: Eventual consistency
        // Store should not have data set on it until event has been published
        // There is an argument we should instead delay reads until the event has been stored but
        // this would:
        //   a. Promote poor patterns of sharing data through persistence
        //   b. Add a large amount of complexity to batching Get operations
        // For now we allow this inconsistency under the assumption that data is written for
        // snapshot storage exclusively.

        // Let's check the eventual consistency all data points should be none...
        assert_eq!(datastore.scope("/foo/name").read::<String>().await?, None);
        assert_eq!(datastore.scope("/foo/age").read::<u64>().await?, None);
        assert_eq!(
            datastore.scope("/foo/occupation").read::<String>().await?,
            None
        );

        // Push an event
        handle.publish(TestEvent::new("pink", 1))?;
        sleep(Duration::from_millis(1)).await;

        // Now we have published an event all data should be written we can get the data from the store
        assert_eq!(
            datastore.scope("/foo/name").read::<String>().await?,
            Some("Fred".to_string())
        );
        assert_eq!(datastore.scope("/foo/age").read::<u64>().await?, Some(21));
        assert_eq!(
            datastore.scope("/foo/occupation").read::<String>().await?,
            Some("developer".to_string())
        );

        // Get a timestamp
        let ts = handle.ts()?;

        // Push a few other events
        handle.publish(TestEvent::new("yellow", 1))?;
        handle.publish(TestEvent::new("red", 1))?;
        handle.publish(TestEvent::new("white", 1))?;
        sleep(Duration::from_millis(100)).await;

        // Get the event logs from the listener
        let logs = listener.send(GetLogs).await?;
        assert_eq!(logs, vec!["pink", "yellow", "red", "white"]);

        // Get the in mem address for the event store
        let es: Addr<EventStore<InMemSequenceIndex, InMemEventLog>> = eventstore.try_into()?;

        // Get all events after the given timestamp and send them to the listener
        es.do_send(GetEventsAfter::new(ts, listener.clone()));
        sleep(Duration::from_millis(100)).await;

        // Pull the events off the listsner since the timestamp
        let events = listener.send(GetEvents).await?;
        assert_eq!(events, vec!["yellow", "red", "white"]);
        Ok(())
    }
}