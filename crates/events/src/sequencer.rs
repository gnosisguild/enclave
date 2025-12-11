// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Actor, Addr, AsyncContext, Handler, Recipient};

use crate::{
    events::{CommitSnapshot, EventStored, StoreEventRequested},
    EnclaveEvent, EventBus, Sequenced, Unsequenced,
};

/// Component to sequence the storage of events
pub struct Sequencer {
    bus: Addr<EventBus<EnclaveEvent<Sequenced>>>,
    seq: u64,
    eventstore: Recipient<StoreEventRequested>,
    snapshot_buffer: Recipient<CommitSnapshot>,
}

impl Sequencer {
    pub fn new(
        bus: &Addr<EventBus<EnclaveEvent<Sequenced>>>,
        eventstore: impl Into<Recipient<StoreEventRequested>>,
        snapshot_buffer: impl Into<Recipient<CommitSnapshot>>,
    ) -> Self {
        Self {
            bus: bus.clone(),
            seq: 0,
            eventstore: eventstore.into(),
            snapshot_buffer: snapshot_buffer.into(),
        }
    }
}

impl Actor for Sequencer {
    type Context = actix::Context<Self>;
}

impl Handler<EnclaveEvent<Unsequenced>> for Sequencer {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent<Unsequenced>, ctx: &mut Self::Context) -> Self::Result {
        self.eventstore
            .do_send(StoreEventRequested::new(msg, ctx.address()))
    }
}

impl Handler<EventStored> for Sequencer {
    type Result = ();
    fn handle(&mut self, msg: EventStored, ctx: &mut Self::Context) -> Self::Result {
        let event = msg.into_event();
        let seq = event.get_seq();
        // TODO: store snapshot...
        self.bus.do_send(event)
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
            bus.publish(d)?;
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
