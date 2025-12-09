// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Actor, Addr, AsyncContext, Handler, Message, Recipient};

use crate::{trap, EType, EnclaveEvent, EventBus, Sequenced, Unsequenced};

#[derive(Message)]
#[rtype(result = "()")]
pub struct PersistRequest {
    pub event: EnclaveEvent<Unsequenced>,
    pub sender: Recipient<EventPersisted>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct EventPersisted {
    pub seq: u64,
    pub event: EnclaveEvent<Unsequenced>,
}

pub struct Sequencer {
    bus: Addr<EventBus<EnclaveEvent<Sequenced>>>,
    seq: u64,
    event_store: Recipient<PersistRequest>,
}

impl Sequencer {
    pub fn new(
        bus: &Addr<EventBus<EnclaveEvent<Sequenced>>>,
        event_store: Recipient<PersistRequest>,
    ) -> Self {
        Self {
            bus: bus.clone(),
            seq: 0,
            event_store,
        }
    }
}

impl Actor for Sequencer {
    type Context = actix::Context<Self>;
}

impl Handler<EnclaveEvent<Unsequenced>> for Sequencer {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent<Unsequenced>, ctx: &mut Self::Context) -> Self::Result {
        self.event_store.do_send(PersistRequest {
            sender: ctx.address().recipient(),
            event: msg,
        })
    }
}

impl Handler<EventPersisted> for Sequencer {
    type Result = ();
    fn handle(&mut self, msg: EventPersisted, ctx: &mut Self::Context) -> Self::Result {
        self.bus.do_send(msg.event.into_sequenced(msg.seq));
    }
}

#[cfg(test)]
mod tests {

    use crate::{prelude::*, BusHandle, EnclaveEvent, EventBus, TakeEvents, TestEvent};
    use actix::Actor;

    #[actix::test]
    async fn it_adds_seqence_numbers_to_events() -> anyhow::Result<()> {
        let bus = BusHandle::new_from_consumer(EventBus::<EnclaveEvent>::default().start());
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
