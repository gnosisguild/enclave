use actix::{Addr, Recipient};

use crate::{ErrorEvent, Event, EventBus, Subscribe};

/// Trait to create events
pub trait EventFactory<E: Event> {
    fn create_local(&self, data: impl Into<E::Data>) -> E;
    fn create_receive(&self, data: impl Into<E::Data>, ts: u128) -> E;
}

/// Trait create errors
pub trait ErrorFactory<E: ErrorEvent> {
    fn create_err(&self, err_type: E::ErrType, error: impl Into<String>) -> E;
}

/// Trait to dispatch events
pub trait EventDispatcher<E: Event> {
    fn dispatch(&self, data: impl Into<E::Data>);
    fn dispatch_from_remote(&self, data: impl Into<E::Data>, ts: u128);
}

/// Trait to subscribe to events
pub trait EventSubscriber<E: Event> {
    fn subscribe(&self, event_type: &str, recipient: Recipient<E>);
}

/// Trait to create an event with a timestamp from its associated type data
pub trait EventConstructorWithTimestamp: Event + Sized {
    fn new_with_timestamp(data: Self::Data, ts: u128) -> Self;
}

pub trait ErrorEventConstructor: ErrorEvent + Sized {
    fn new_error(err_type: Self::ErrType, error: impl Into<String>) -> Self;
}

pub trait ManagedEvent: ErrorEvent + EventConstructorWithTimestamp + ErrorEventConstructor {}

impl<E> ManagedEvent for E where
    E: ErrorEvent + EventConstructorWithTimestamp + ErrorEventConstructor
{
}

#[derive(Clone)]
pub struct EventManager<E: Event> {
    bus: Addr<EventBus<E>>,
}

impl<E: Event> EventManager<E> {
    pub fn new(bus: Addr<EventBus<E>>) -> Self {
        Self { bus }
    }
}

impl<E> EventDispatcher<E> for EventManager<E>
where
    E: ManagedEvent,
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

impl<E: EventConstructorWithTimestamp> EventFactory<E> for EventManager<E> {
    fn create_local(&self, data: impl Into<E::Data>) -> E {
        E::new_with_timestamp(data.into(), 0)
    }

    fn create_receive(&self, data: impl Into<E::Data>, ts: u128) -> E {
        E::new_with_timestamp(data.into(), ts)
    }
}

impl<E: ErrorEventConstructor> ErrorFactory<E> for EventManager<E> {
    fn create_err(&self, err_type: E::ErrType, error: impl Into<String>) -> E {
        E::new_error(err_type, error)
    }
}

impl<E: Event> EventSubscriber<E> for EventManager<E> {
    fn subscribe(&self, event_type: &str, recipient: Recipient<E>) {
        self.bus.do_send(Subscribe::new(event_type, recipient))
    }
}
