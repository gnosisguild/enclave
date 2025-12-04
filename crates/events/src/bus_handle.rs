// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::sync::Arc;

use actix::{Actor, Addr, Recipient};
use anyhow::Result;
use derivative::Derivative;

use crate::{
    hlc::Hlc,
    sequencer::Sequencer,
    traits::{
        ErrorDispatcher, ErrorFactory, EventConstructorWithTimestamp, EventFactory, EventPublisher,
        EventSubscriber,
    },
    EType, EnclaveEvent, EnclaveEventData, ErrorEvent, EventBus, HistoryCollector, Stored,
    Subscribe, Unstored,
};

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct BusHandle {
    consumer: Addr<EventBus<EnclaveEvent<Stored>>>,
    producer: Addr<Sequencer>,
    #[derivative(Debug = "ignore")]
    hlc: Arc<Hlc>,
}

impl BusHandle {
    pub fn new(consumer: Addr<EventBus<EnclaveEvent<Stored>>>) -> Self {
        let producer = Sequencer::new(&consumer).start();
        let hlc = Hlc::default();
        Self {
            consumer,
            producer,
            hlc: Arc::new(hlc),
        }
    }

    pub fn history(&self) -> Addr<HistoryCollector<EnclaveEvent<Stored>>> {
        EventBus::<EnclaveEvent<Stored>>::history(&self.consumer)
    }

    pub fn producer(&self) -> &Addr<Sequencer> {
        &self.producer
    }

    pub fn consumer(&self) -> &Addr<EventBus<EnclaveEvent<Stored>>> {
        &self.consumer
    }
}

impl EventPublisher<EnclaveEvent<Unstored>> for BusHandle {
    fn publish(&self, data: impl Into<EnclaveEventData>) -> Result<()> {
        let evt = self.event_from(data)?;
        self.producer.do_send(evt);
        Ok(())
    }

    fn publish_from_remote(&self, data: impl Into<EnclaveEventData>, ts: u128) -> Result<()> {
        let evt = self.event_from_remote_source(data, ts)?;
        self.producer.do_send(evt);
        Ok(())
    }

    fn naked_dispatch(&self, event: EnclaveEvent<Unstored>) {
        self.producer.do_send(event);
    }
}

impl ErrorDispatcher<EnclaveEvent<Unstored>> for BusHandle {
    fn err(&self, err_type: EType, error: impl Into<anyhow::Error>) {
        let evt = self.event_from_error(err_type, error);
        self.producer.do_send(evt);
    }
}

impl EventFactory<EnclaveEvent<Unstored>> for BusHandle {
    fn event_from(&self, data: impl Into<EnclaveEventData>) -> Result<EnclaveEvent<Unstored>> {
        let ts = self.hlc.tick()?;
        Ok(EnclaveEvent::<Unstored>::new_with_timestamp(
            data.into(),
            ts.into(),
        ))
    }

    fn event_from_remote_source(
        &self,
        data: impl Into<EnclaveEventData>,
        ts: u128,
    ) -> Result<EnclaveEvent<Unstored>> {
        let ts = self.hlc.receive(&ts.into())?;
        Ok(EnclaveEvent::<Unstored>::new_with_timestamp(
            data.into(),
            ts.into(),
        ))
    }
}

impl ErrorFactory<EnclaveEvent<Unstored>> for BusHandle {
    fn event_from_error(
        &self,
        err_type: EType,
        error: impl Into<anyhow::Error>,
    ) -> EnclaveEvent<Unstored> {
        EnclaveEvent::<Unstored>::from_error(err_type, error)
    }
}

impl EventSubscriber<EnclaveEvent<Stored>> for BusHandle {
    fn subscribe(&self, event_type: &str, recipient: Recipient<EnclaveEvent<Stored>>) {
        self.consumer.do_send(Subscribe::new(event_type, recipient))
    }

    fn subscribe_all(&self, event_types: &[&str], recipient: Recipient<EnclaveEvent<Stored>>) {
        for event_type in event_types.into_iter() {
            self.consumer
                .do_send(Subscribe::new(*event_type, recipient.clone()));
        }
    }
}

impl Into<BusHandle> for Addr<EventBus<EnclaveEvent>> {
    fn into(self) -> BusHandle {
        BusHandle::new(self)
    }
}

impl Into<BusHandle> for &Addr<EventBus<EnclaveEvent>> {
    fn into(self) -> BusHandle {
        BusHandle::new(self.clone())
    }
}
