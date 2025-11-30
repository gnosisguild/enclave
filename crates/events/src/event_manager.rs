// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Addr, Recipient};

use crate::{
    traits::{
        CompositeEvent, ErrorDispatcher, ErrorFactory, Event, EventConstructorWithTimestamp,
        EventDispatcher, EventFactory, EventSubscriber,
    },
    ErrorEvent, EventBus, Subscribe,
};

#[derive(Clone, Debug)]
pub struct BusHandle<E: Event> {
    bus: Addr<EventBus<E>>,
}

impl<E: Event> BusHandle<E> {
    pub fn new(bus: Addr<EventBus<E>>) -> Self {
        Self { bus }
    }

    pub fn bus(&self) -> Addr<EventBus<E>> {
        self.bus.clone()
    }
}

impl<E: CompositeEvent> EventDispatcher<E> for BusHandle<E> {
    fn dispatch(&self, data: impl Into<E::Data>) {
        let evt = self.event_from(data);
        self.bus.do_send(evt);
    }

    fn dispatch_from_remote(&self, data: impl Into<E::Data>, ts: u128) {
        let evt = self.create_receive(data, ts);
        self.bus.do_send(evt)
    }

    fn naked_dispatch(&self, event: E) {
        self.bus.do_send(event);
    }
}

impl<E> ErrorDispatcher<E> for BusHandle<E>
where
    E: CompositeEvent,
{
    fn err(&self, err_type: E::ErrType, error: impl Into<E::FromError>) {
        let evt = self.create_err(err_type, error);
        self.bus.do_send(evt);
    }
}

impl<E: EventConstructorWithTimestamp> EventFactory<E> for BusHandle<E> {
    fn event_from(&self, data: impl Into<E::Data>) -> E {
        E::new_with_timestamp(data.into(), 0)
    }

    fn create_receive(&self, data: impl Into<E::Data>, ts: u128) -> E {
        E::new_with_timestamp(data.into(), ts)
    }
}

impl<E: ErrorEvent> ErrorFactory<E> for BusHandle<E> {
    fn create_err(&self, err_type: E::ErrType, error: impl Into<E::FromError>) -> E {
        E::from_error(err_type, error)
    }
}

impl<E: Event> EventSubscriber<E> for BusHandle<E> {
    fn subscribe(&self, event_type: &str, recipient: Recipient<E>) {
        self.bus.do_send(Subscribe::new(event_type, recipient))
    }

    fn subscribe_all(&self, event_types: &[&str], recipient: Recipient<E>) {
        for event_type in event_types.into_iter() {
            self.bus
                .do_send(Subscribe::new(*event_type, recipient.clone()));
        }
    }
}
