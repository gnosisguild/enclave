use actix::{Addr, Recipient};

use crate::{
    traits::{
        ErrorDispatcher, ErrorEventConstructor, ErrorFactory, Event, EventConstructorWithTimestamp,
        EventDispatcher, EventFactory, EventSubscriber, ManagedEvent,
    },
    EventBus, Subscribe,
};

#[derive(Clone)]
pub struct EventManager<E: Event> {
    bus: Addr<EventBus<E>>,
}

impl<E: Event> EventManager<E> {
    pub fn new(bus: Addr<EventBus<E>>) -> Self {
        Self { bus }
    }
}

impl<E: ManagedEvent> EventDispatcher<E> for EventManager<E> {
    fn dispatch(&self, data: impl Into<E::Data>) {
        let evt = self.create_local(data);
        self.bus.do_send(evt);
    }

    fn dispatch_from_remote(&self, data: impl Into<E::Data>, ts: u128) {
        let evt = self.create_receive(data, ts);
        self.bus.do_send(evt)
    }
}

impl<E> ErrorDispatcher<E> for EventManager<E>
where
    E: ManagedEvent,
{
    fn err(&self, err_type: E::ErrType, error: impl Into<E::FromError>) {
        let evt = self.create_err(err_type, error);
        self.bus.do_send(evt);
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
    fn create_err(&self, err_type: E::ErrType, error: impl Into<E::FromError>) -> E {
        E::new_error(err_type, error)
    }
}

impl<E: Event> EventSubscriber<E> for EventManager<E> {
    fn subscribe(&self, event_type: &str, recipient: Recipient<E>) {
        self.bus.do_send(Subscribe::new(event_type, recipient))
    }
}
