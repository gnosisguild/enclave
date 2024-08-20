use crate::events::{EnclaveEvent, EventId};
use actix::prelude::*;
use std::collections::{HashMap, HashSet};

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
            .or_insert_with(Vec::new)
            .push(event.listener);
    }
}

impl Handler<GetHistory> for EventBus {
    type Result = Vec<EnclaveEvent>;

    fn handle(&mut self, _: GetHistory, _: &mut Context<Self>) -> Vec<EnclaveEvent> {
        self.history.clone()
    }
}

impl Handler<EnclaveEvent> for EventBus {
    type Result = ();

    fn handle(&mut self, event: EnclaveEvent, _: &mut Context<Self>) {
        // Deduplicate by id
        if self.ids.contains(&event.clone().into()) {
            // We have seen this before
            println!("Duplicate {}", EventId::from(event));
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
