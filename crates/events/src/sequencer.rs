// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{
    events::{StoreEventRequested, StoreEventResponse},
    EnclaveEvent, EventBus, Sequenced, Unsequenced,
};
use actix::{Actor, Addr, AsyncContext, Handler, Recipient};
use anyhow::Result;
use e3_utils::major_issue;

/// Component to sequence the storage of events
pub struct Sequencer {
    bus: Addr<EventBus<EnclaveEvent<Sequenced>>>,
    eventstore: Recipient<StoreEventRequested>,
    buffer: Recipient<EnclaveEvent>,
}

impl Sequencer {
    pub fn new(
        bus: &Addr<EventBus<EnclaveEvent<Sequenced>>>,
        eventstore: impl Into<Recipient<StoreEventRequested>>,
        buffer: impl Into<Recipient<EnclaveEvent>>,
    ) -> Self {
        Self {
            bus: bus.clone(),
            eventstore: eventstore.into(),
            buffer: buffer.into(),
        }
    }

    fn handle_store_event_response(&self, msg: StoreEventResponse) -> Result<()> {
        let event = msg.into_event();
        self.buffer.try_send(event.clone())?;
        self.bus.try_send(event)?;
        Ok(())
    }
}

impl Actor for Sequencer {
    type Context = actix::Context<Self>;
}

impl Handler<EnclaveEvent<Unsequenced>> for Sequencer {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent<Unsequenced>, ctx: &mut Self::Context) -> Self::Result {
        if let Err(e) = self
            .eventstore
            .try_send(StoreEventRequested::new(msg, ctx.address()))
        {
            panic!("{}", major_issue("Could not store event in eventstore.", e))
        }
    }
}

impl Handler<StoreEventResponse> for Sequencer {
    type Result = ();
    fn handle(&mut self, msg: StoreEventResponse, _: &mut Self::Context) -> Self::Result {
        if let Err(e) = self.handle_store_event_response(msg) {
            panic!(
                "{}",
                major_issue("Could not send event to snapshot_buffer or bus.", e)
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use e3_ciphernode_builder::EventSystem;
    use e3_events::{EnclaveEvent, EventPublisher, TakeEvents, TestEvent};

    #[actix::test]
    async fn it_adds_seqence_numbers_to_events() -> anyhow::Result<()> {
        let system = EventSystem::new("test");
        let bus = system.handle()?;
        let history = bus.history();

        let event_data = vec![
            TestEvent::new("one", 1),
            TestEvent::new("two", 2),
            TestEvent::new("three", 3),
        ];

        for d in event_data.clone() {
            bus.publish_without_context(d)?;
        }

        let expected = event_data
            .into_iter()
            .map(|d| EnclaveEvent::new_stored_event(d.clone().into(), 0, d.entropy))
            .collect::<Vec<_>>();
        let events = history.send(TakeEvents::new(3)).await?;

        assert_eq!(
            events
                .iter()
                .map(EnclaveEvent::strip_ts)
                .collect::<Vec<_>>(),
            expected
        );
        Ok(())
    }
}
