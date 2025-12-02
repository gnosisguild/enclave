// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::marker::PhantomData;

use actix::{Actor, Addr, Recipient};

use crate::{
    sequencer::Sequencer,
    traits::{
        ErrorDispatcher, ErrorFactory, Event, EventConstructorWithTimestamp, EventFactory,
        EventPublisher, EventSubscriber,
    },
    EnclaveErrorType, EnclaveEvent, EnclaveEventData, ErrorEvent, EventBus, Stored, Subscribe,
    Unstored,
};

#[derive(Clone, Debug)]
pub struct BusHandle {
    bus: Addr<EventBus<EnclaveEvent<Stored>>>,
    seq: Addr<Sequencer>,
}

impl BusHandle {
    pub fn new(bus: Addr<EventBus<EnclaveEvent<Stored>>>) -> Self {
        let seq = Sequencer::new(&bus).start();
        Self { bus, seq }
    }

    pub fn bus(&self) -> Addr<EventBus<E>> {
        self.bus.clone()
    }
}

impl EventPublisher<EnclaveEvent<Unstored>> for BusHandle {
    fn publish(&self, data: impl Into<EnclaveEventData>) {
        let evt = self.event_from(data);
        self.seq.do_send(evt);
    }

    fn publish_from_remote(&self, data: impl Into<EnclaveEventData>, ts: u128) {
        let evt = self.event_from_remote_source(data, ts);
        self.seq.do_send(evt)
    }

    fn naked_dispatch(&self, event: EnclaveEvent<Unstored>) {
        self.seq.do_send(event);
    }
}

impl ErrorDispatcher<EnclaveEvent<Unstored>> for BusHandle {
    fn err(&self, err_type: EnclaveErrorType, error: impl Into<anyhow::Error>) {
        let evt = self.event_from_error(err_type, error);
        self.seq.do_send(evt);
    }
}

impl EventFactory<EnclaveEvent<Unstored>> for BusHandle {
    fn event_from(&self, data: impl Into<EnclaveEventData>) -> EnclaveEvent<Unstored> {
        // TODO: add self.hcl.tick()
        EnclaveEvent::<Unstored>::new_with_timestamp(data.into(), 0)
    }

    fn event_from_remote_source(&self, data: impl Into<E::Data>, ts: u128) -> E {
        // TODO: add self.hcl.receive(ts)
        EnclaveEvent::<Unstored>::new_with_timestamp(data.into(), ts)
    }
}

impl ErrorFactory<EnclaveEvent<Unstored>> for BusHandle {
    fn event_from_error(&self, err_type: EnclaveErrorType, error: impl Into<anyhow::Error>) -> E {
        EnclaveEvent::<Unstored>::from_error(err_type, error)
    }
}

impl<E: Event> EventSubscriber<EnclaveEvent<Stored>> for BusHandle {
    fn subscribe(&self, event_type: &str, recipient: Recipient<EnclaveEvent<Stored>>) {
        self.bus.do_send(Subscribe::new(event_type, recipient))
    }

    fn subscribe_all(&self, event_types: &[&str], recipient: Recipient<EnclaveEvent<Stored>>) {
        for event_type in event_types.into_iter() {
            self.bus
                .do_send(Subscribe::new(*event_type, recipient.clone()));
        }
    }
}
