use std::sync::Arc;

use actix::{Addr, Recipient};

use crate::{ErrorEvent, Event, EventBus, Subscribe};

pub trait EventFactory<E: Event> {
    fn create_local(&self, data: impl Into<E::Data>) -> E;
    fn create_receive(&self, data: impl Into<E::Data>, ts: u128) -> E;
}

pub trait ErrorFactory<E: ErrorEvent> {
    fn create_err(&self, err_type: E::ErrType, error: impl Into<String>) -> E;
}

pub trait EventDispatcher<E: Event> {
    fn dispatch(&self, data: impl Into<E::Data>);
    fn dispatch_from_remote(&self, data: impl Into<E::Data>, ts: u128);
}

pub trait EventSubscriber<E: Event> {
    fn subscribe(&self, event_type: &str, recipient: Recipient<E>);
}

pub trait FullFactory<E: ErrorEvent>: ErrorFactory<E> + EventFactory<E> {}

pub struct EventManager<E: Event, F> {
    bus: Addr<EventBus<E>>,
    factory: Arc<F>,
}

impl<E: Event, F> EventManager<E, F> {
    pub fn new(bus: Addr<EventBus<E>>, factory: Arc<F>) -> Self {
        Self { bus, factory }
    }
}

impl<E, F> EventDispatcher<E> for EventManager<E, F>
where
    E: Event,
    F: EventFactory<E>,
{
    fn dispatch(&self, data: impl Into<E::Data>) {
        let evt = self.create_local(data);
        self.bus.do_send(evt);
    }

    fn dispatch_from_remote(&self, data: impl Into<E::Data>, ts: u128) {
        let evt = self.create_receive(data, ts);
        self.bus.do_send(evt)
    }
}

impl<E, F> EventFactory<E> for EventManager<E, F>
where
    E: Event,
    F: EventFactory<E>,
{
    fn create_local(&self, data: impl Into<E::Data>) -> E {
        self.factory.create_local(data)
    }

    fn create_receive(&self, data: impl Into<E::Data>, ts: u128) -> E {
        self.factory.create_receive(data, ts)
    }
}

impl<E: ErrorEvent, F: ErrorFactory<E>> ErrorFactory<E> for EventManager<E, F> {
    fn create_err(&self, err_type: E::ErrType, error: impl Into<String>) -> E {
        self.factory.create_err(err_type, error)
    }
}

impl<E: Event, F> EventSubscriber<E> for EventManager<E, F> {
    fn subscribe(&self, event_type: &str, recipient: Recipient<E>) {
        self.bus.do_send(Subscribe::new(event_type, recipient))
    }
}

impl<E: ErrorEvent, F: FullFactory<E>> FullFactory<E> for EventManager<E, F> {}
