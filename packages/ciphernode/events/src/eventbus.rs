use actix::prelude::*;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::marker::PhantomData;

//////////////////////////////////////////////////////////////////////////////
// Core Traits
//////////////////////////////////////////////////////////////////////////////

/// Trait that must be implemented by events used with EventBus
pub trait Event: Message<Result = ()> + Clone + Send + Sync + Unpin + 'static {
    type Id: Hash + Eq + Clone + Unpin;
    fn event_type(&self) -> String;
    fn event_id(&self) -> Self::Id;
}

/// Trait for events that contain an error
pub trait ErrorEvent: Event {
    type Error: Clone;
    type ErrorType;

    fn as_error(&self) -> Option<&Self::Error>;
    fn from_error(err_type: Self::ErrorType, error: anyhow::Error) -> Self;
}

//////////////////////////////////////////////////////////////////////////////
// Configuration
//////////////////////////////////////////////////////////////////////////////

/// Configuration for EventBus behavior
pub struct EventBusConfig {
    pub capture_history: bool,
    pub deduplicate: bool,
}

impl Default for EventBusConfig {
    fn default() -> Self {
        Self {
            capture_history: true,
            deduplicate: true,
        }
    }
}

//////////////////////////////////////////////////////////////////////////////
// EventBus Implementation
//////////////////////////////////////////////////////////////////////////////
/// Central EventBus for each node. Actors publish events to this bus by sending it EnclaveEvents.
/// All events sent to this bus are assumed to be published over the network via pubsub.
/// Other actors such as the NetworkManager and Evm actor connect to outside services and control which events
/// actually get published as well as ensure that local events are not rebroadcast locally after
/// being published.
pub struct EventBus<E: Event> {
    config: EventBusConfig,
    history: Vec<E>,
    ids: HashSet<E::Id>,
    listeners: HashMap<String, Vec<Recipient<E>>>,
}

impl<E: Event> Actor for EventBus<E> {
    type Context = Context<Self>;
}

impl<E: Event> EventBus<E> {
    pub fn new(config: EventBusConfig) -> Self {
        EventBus {
            config,
            listeners: HashMap::new(),
            ids: HashSet::new(),
            history: vec![],
        }
    }

    fn add_to_history(&mut self, event: E) {
        if self.config.capture_history {
            self.history.push(event.clone());
        }
        if self.config.deduplicate {
            self.ids.insert(event.event_id());
        }
    }

    fn is_duplicate(&self, event: &E) -> bool {
        self.config.deduplicate && self.ids.contains(&event.event_id())
    }
}

//////////////////////////////////////////////////////////////////////////////
// Subscribe Message
//////////////////////////////////////////////////////////////////////////////

#[derive(Message)]
#[rtype(result = "()")]
pub struct Subscribe<E: Event> {
    pub event_type: String,
    pub listener: Recipient<E>,
}

impl<E: Event> Subscribe<E> {
    pub fn new(event_type: impl Into<String>, listener: Recipient<E>) -> Self {
        Self {
            event_type: event_type.into(),
            listener,
        }
    }
}

impl<E: Event> Handler<Subscribe<E>> for EventBus<E> {
    type Result = ();

    fn handle(&mut self, msg: Subscribe<E>, _: &mut Context<Self>) {
        self.listeners
            .entry(msg.event_type)
            .or_default()
            .push(msg.listener);
    }
}

//////////////////////////////////////////////////////////////////////////////
// History Management
//////////////////////////////////////////////////////////////////////////////

#[derive(Message)]
#[rtype(result = "Vec<E>")]
pub struct GetHistory<E: Event>(PhantomData<E>);

impl<E: Event> GetHistory<E> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<E: Event> Handler<GetHistory<E>> for EventBus<E> {
    type Result = Vec<E>;

    fn handle(&mut self, _: GetHistory<E>, _: &mut Context<Self>) -> Vec<E> {
        self.history.clone()
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ResetHistory;

impl<E: Event> Handler<ResetHistory> for EventBus<E> {
    type Result = ();

    fn handle(&mut self, _: ResetHistory, _: &mut Context<Self>) {
        self.history.clear()
    }
}

//////////////////////////////////////////////////////////////////////////////
// Error Handling
//////////////////////////////////////////////////////////////////////////////

#[derive(Message)]
#[rtype(result = "Vec<E::Error>")]
pub struct GetErrors<E: ErrorEvent>(PhantomData<E>);

impl<E: ErrorEvent> GetErrors<E> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<E: ErrorEvent> Handler<GetErrors<E>> for EventBus<E> {
    type Result = Vec<E::Error>;

    fn handle(&mut self, _: GetErrors<E>, _: &mut Context<Self>) -> Vec<E::Error> {
        self.history
            .iter()
            .filter_map(|evt| evt.as_error())
            .cloned()
            .collect()
    }
}

//////////////////////////////////////////////////////////////////////////////
// Event Handling
//////////////////////////////////////////////////////////////////////////////

impl<E: Event> Handler<E> for EventBus<E> {
    type Result = ();

    fn handle(&mut self, event: E, _: &mut Context<Self>) {
        if self.is_duplicate(&event) {
            return;
        }

        if let Some(listeners) = self.listeners.get("*") {
            for listener in listeners {
                listener.do_send(event.clone())
            }
        }

        if let Some(listeners) = self.listeners.get(&event.event_type()) {
            for listener in listeners {
                listener.do_send(event.clone())
            }
        }

        self.add_to_history(event);
    }
}

//////////////////////////////////////////////////////////////////////////////
// Error Bus Trait
//////////////////////////////////////////////////////////////////////////////

/// Trait to send errors directly to the bus
pub trait BusError<E: ErrorEvent> {
    fn err(&self, err_type: E::ErrorType, err: anyhow::Error);
}

impl<E: ErrorEvent> BusError<E> for Addr<EventBus<E>> {
    fn err(&self, err_type: E::ErrorType, err: anyhow::Error) {
        self.do_send(E::from_error(err_type, err))
    }
}

impl<E: ErrorEvent> BusError<E> for Recipient<E> {
    fn err(&self, err_type: E::ErrorType, err: anyhow::Error) {
        self.do_send(E::from_error(err_type, err))
    }
}