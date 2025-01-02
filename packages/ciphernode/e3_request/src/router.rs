use crate::ContextRepositoryFactory;
use crate::E3Context;
use crate::E3ContextParams;
use crate::E3ContextSnapshot;
use crate::E3MetaExtension;
use crate::RouterRepositoryFactory;
use actix::AsyncContext;
use actix::{Actor, Addr, Context, Handler};
use anyhow::*;
use async_trait::async_trait;
use data::Checkpoint;
use data::DataStore;
use data::FromSnapshotWithParams;
use data::RepositoriesFactory;
use data::Repository;
use data::Snapshot;
use events::E3RequestComplete;
use events::Shutdown;
use events::{E3id, EnclaveEvent, EventBus, Subscribe};
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashSet;
use std::{collections::HashMap, sync::Arc};
use tracing::error;

/// Buffers events for downstream instances to handle out-of-order event delivery.
/// Events are stored in a HashMap keyed by string identifiers until they are ready
/// to be processed.
#[derive(Default)]
pub struct EventBuffer {
    buffer: HashMap<String, Vec<EnclaveEvent>>,
}

impl EventBuffer {
    pub fn add(&mut self, key: &str, event: EnclaveEvent) {
        self.buffer.entry(key.to_string()).or_default().push(event)
    }

    pub fn take(&mut self, key: &str) -> Vec<EnclaveEvent> {
        self.buffer
            .get_mut(key)
            .map(std::mem::take)
            .unwrap_or_default()
    }
}

/// An Extension interface for the E3Router system that listens and responds to EnclaveEvents.
///
/// # Responsibilities
/// - Listens for broadcast EnclaveEvents
/// - Instantiates appropriate actors based on received events
/// - Manages actor state persistence and reconstruction
/// - Handles event streaming to registered recipients
///
/// # Usage
/// Extensions implement the `on_event` handler to define which events they respond to.
/// When an event is received, the extension typically:
/// 1. Uses the request's context to construct required actors
/// 2. Saves actor addresses to the context using `set_event_recipient`
/// 3. Manages event streaming from buffers to registered recipients
///
/// Extensions can also reconstruct actors from persisted state using context
/// snapshots and repositories. They can check for dependencies in the context
/// before constructing new extensions.
#[async_trait]
pub trait E3Extension: Send + Sync + 'static {
    /// This function is triggered when an EnclaveEvent is sent to the router. Use this to
    /// initialize the receiver using `ctx.set_event_receiver(my_address.into())`. Typically this
    /// means filtering for specific e3_id enabled events that give rise to actors that have to
    /// handle certain behaviour.
    fn on_event(&self, ctx: &mut E3Context, evt: &EnclaveEvent);

    /// This function it triggered when the request context is being hydrated from snapshot.
    async fn hydrate(&self, ctx: &mut E3Context, snapshot: &E3ContextSnapshot) -> Result<()>;
}

/// Routes E3_id-specific contexts to registered extensions and manages message forwarding.
///
/// # Core Functions
/// - Maintains contexts for each E3 request
/// - Lazily registers extension instances with appropriate dependencies per E3_id
/// - Forwards incoming messages to registered instances
/// - Manages request lifecycle and completion
///
/// Extensions receive an E3_id-specific context and can handle specific
/// message types. The router ensures proper message delivery and context management
/// throughout the request lifecycle.
// TODO: setup so that we have to place extensions within correct order of dependencies
pub struct E3Router {
    /// The context for every E3 request
    contexts: HashMap<E3id, E3Context>,
    /// A list of completed requests
    completed: HashSet<E3id>,
    /// The extensions this instance of the router is configured to listen for
    extensions: Arc<Vec<Box<dyn E3Extension>>>,
    /// A buffer for events to send to the
    buffer: EventBuffer,
    /// The EventBus
    bus: Addr<EventBus>,
    /// A repository for storing snapshots
    store: Repository<E3RouterSnapshot>,
}

pub struct E3RouterParams {
    extensions: Arc<Vec<Box<dyn E3Extension>>>,
    bus: Addr<EventBus>,
    store: Repository<E3RouterSnapshot>,
}

impl E3Router {
    pub fn builder(bus: &Addr<EventBus>, store: DataStore) -> E3RouterBuilder {
        let repositories = store.repositories();
        let builder = E3RouterBuilder {
            bus: bus.clone(),
            extensions: vec![],
            store: repositories.router(),
        };

        // Everything needs the committe meta factory so adding it here by default
        builder.with(E3MetaExtension::create())
    }

    pub fn from_params(params: E3RouterParams) -> Self {
        Self {
            extensions: params.extensions,
            bus: params.bus.clone(),
            store: params.store.clone(),
            completed: HashSet::new(),
            contexts: HashMap::new(),
            buffer: EventBuffer {
                buffer: HashMap::new(),
            },
        }
    }
}

impl Actor for E3Router {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for E3Router {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        // If we are shuttomg down then bail on anything else
        if let EnclaveEvent::Shutdown { data, .. } = msg {
            ctx.notify(data);
            return;
        }

        // Only process events with e3_ids
        let Some(e3_id) = msg.get_e3_id() else {
            return;
        };

        // If this e3_id has already been completed then we are not going to do anything here
        if self.completed.contains(&e3_id) {
            error!("Received the following event to E3Id({}) despite already being completed:\n\n{:?}\n\n", e3_id, msg);
            return;
        }

        let repositories = self.repository().repositories();
        let context = self.contexts.entry(e3_id.clone()).or_insert_with(|| {
            E3Context::from_params(E3ContextParams {
                e3_id: e3_id.clone(),
                repository: repositories.context(&e3_id),
                extensions: self.extensions.clone(),
            })
        });

        for extension in self.extensions.iter() {
            extension.on_event(context, &msg);
        }

        context.forward_message(&msg, &mut self.buffer);

        match &msg {
            EnclaveEvent::PlaintextAggregated { .. } => {
                // Here we are detemining that by receiving the PlaintextAggregated event our request is
                // complete and we can notify everyone. This might change as we consider other factors
                // when determining if the request is complete
                let event = EnclaveEvent::from(E3RequestComplete {
                    e3_id: e3_id.clone(),
                });

                // Send to bus so all other actors can react to a request being complete.
                self.bus.do_send(event);
            }
            EnclaveEvent::E3RequestComplete { .. } => {
                // Note this will be sent above to the children who can kill themselves based on
                // the event
                self.contexts.remove(&e3_id);
                self.completed.insert(e3_id);
            }
            _ => (),
        }

        self.checkpoint();
    }
}

impl Handler<Shutdown> for E3Router {
    type Result = ();
    fn handle(&mut self, msg: Shutdown, _ctx: &mut Self::Context) -> Self::Result {
        let shutdown_evt = EnclaveEvent::from(msg);
        for (_, ctx) in self.contexts.iter() {
            ctx.forward_message_now(&shutdown_evt)
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct E3RouterSnapshot {
    contexts: Vec<E3id>,
    completed: HashSet<E3id>,
}

impl Snapshot for E3Router {
    type Snapshot = E3RouterSnapshot;
    fn snapshot(&self) -> Result<Self::Snapshot> {
        let contexts = self.contexts.keys().cloned().collect();
        let completed = self.completed.clone();

        Ok(Self::Snapshot {
            completed,
            contexts,
        })
    }
}

impl Checkpoint for E3Router {
    fn repository(&self) -> &Repository<E3RouterSnapshot> {
        &self.store
    }
}

#[async_trait]
impl FromSnapshotWithParams for E3Router {
    type Params = E3RouterParams;

    async fn from_snapshot(params: Self::Params, snapshot: Self::Snapshot) -> Result<Self> {
        let mut contexts = HashMap::new();

        let repositories = params.store.repositories();
        for e3_id in snapshot.contexts {
            let Some(ctx_snapshot) = repositories.context(&e3_id).read().await? else {
                continue;
            };

            contexts.insert(
                e3_id.clone(),
                E3Context::from_snapshot(
                    E3ContextParams {
                        repository: repositories.context(&e3_id),
                        e3_id: e3_id.clone(),
                        extensions: params.extensions.clone(),
                    },
                    ctx_snapshot,
                )
                .await?,
            );
        }

        Ok(E3Router {
            contexts,
            completed: snapshot.completed,
            extensions: params.extensions.into(),
            buffer: EventBuffer::default(),
            bus: params.bus,
            store: repositories.router(),
        })
    }
}

/// Builder for E3Router
pub struct E3RouterBuilder {
    pub bus: Addr<EventBus>,
    pub extensions: Vec<Box<dyn E3Extension>>,
    pub store: Repository<E3RouterSnapshot>,
}

impl E3RouterBuilder {
    pub fn with(mut self, listener: Box<dyn E3Extension>) -> Self {
        self.extensions.push(listener);
        self
    }

    pub async fn build(self) -> Result<Addr<E3Router>> {
        let repositories = self.store.repositories();
        let router_repo = repositories.router();
        let snapshot: Option<E3RouterSnapshot> = router_repo.read().await?;
        let params = E3RouterParams {
            extensions: self.extensions.into(),
            bus: self.bus.clone(),

            store: router_repo,
        };

        let e3r = match snapshot {
            Some(snapshot) => E3Router::from_snapshot(params, snapshot).await?,
            None => E3Router::from_params(params),
        };

        let addr = e3r.start();
        self.bus
            .do_send(Subscribe::new("*", addr.clone().recipient()));
        Ok(addr)
    }
}
