use actix::prelude::*;
use std::collections::{HashMap, HashSet};

use crate::EnclaveErrorType;

use super::events::{EnclaveEvent, EventId, FromError};

#[derive(Message, Debug)]
#[rtype(result = "()")]
pub struct Subscribe {
    pub event_type: String,
    pub listener: Recipient<EnclaveEvent>,
}

impl Subscribe {
    pub fn new(event_type: impl Into<String>, listener: Recipient<EnclaveEvent>) -> Self {
        Self {
            event_type: event_type.into(),
            listener,
        }
    }
}

#[derive(Message)]
#[rtype(result = "Vec<EnclaveEvent>")]
pub struct GetHistory;

#[derive(Message)]
#[rtype(result = "()")]
pub struct ResetHistory;

/// Central EventBus for each node. Actors publish events to this bus by sending it EnclaveEvents.
/// All events sent to this bus are assumed to be published over the network via pubsub.
/// Other actors such as the P2p and Evm actor connect to outside services and control which events
/// actually get published as well as ensure that local events are not rebroadcast locally after
/// being published.
pub struct EventBus {
    capture: bool,
    history: Vec<EnclaveEvent>,
    ids: HashSet<EventId>,
    listeners: HashMap<String, Vec<Recipient<EnclaveEvent>>>,
}

impl Actor for EventBus {
    type Context = Context<Self>;
}

impl EventBus {
    pub fn new(capture: bool) -> Self {
        EventBus {
            capture,
            listeners: HashMap::new(),
            ids: HashSet::new(),
            history: vec![],
        }
    }

    fn add_to_history(&mut self, event: EnclaveEvent) {
        self.history.push(event.clone());
        self.ids.insert(event.into());
    }
}

impl Handler<Subscribe> for EventBus {
    type Result = ();

    fn handle(&mut self, event: Subscribe, _: &mut Context<Self>) {
        self.listeners
            .entry(event.event_type)
            .or_default()
            .push(event.listener);
    }
}

impl Handler<GetHistory> for EventBus {
    type Result = Vec<EnclaveEvent>;

    fn handle(&mut self, _: GetHistory, _: &mut Context<Self>) -> Vec<EnclaveEvent> {
        self.history.clone()
    }
}
impl Handler<ResetHistory> for EventBus {
    type Result = ();

    fn handle(&mut self, _: ResetHistory, _: &mut Context<Self>) {
        self.history.clear()
    }
}

impl Handler<EnclaveEvent> for EventBus {
    type Result = ();

    fn handle(&mut self, event: EnclaveEvent, _: &mut Context<Self>) {
        // Deduplicate by id
        if self.ids.contains(&event.get_id()) {
            // We have seen this before
            return;
        }

        // TODO: How can we ensure the event we see is coming in in the correct order?
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

        if self.capture {
            self.add_to_history(event);
        }
    }
}

/// Trait to send errors directly to the bus
pub trait BusError {
    fn err(&self, err_type: EnclaveErrorType, err: anyhow::Error);
}

impl BusError for Addr<EventBus> {
    fn err(&self, err_type: EnclaveErrorType, err: anyhow::Error) {
        self.do_send(EnclaveEvent::from_error(err_type, err))
    }
}
impl BusError for Recipient<EnclaveEvent> {
    fn err(&self, err_type: EnclaveErrorType, err: anyhow::Error) {
        self.do_send(EnclaveEvent::from_error(err_type, err))
    }
}
