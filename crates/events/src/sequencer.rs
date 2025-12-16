// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Actor, Addr, Handler};

use crate::{EnclaveEvent, EventBus, Sequenced, Unsequenced};

pub struct Sequencer {
    bus: Addr<EventBus<EnclaveEvent<Sequenced>>>,
    seq: u64,
}

impl Sequencer {
    pub fn new(bus: &Addr<EventBus<EnclaveEvent<Sequenced>>>) -> Self {
        Self {
            bus: bus.clone(),
            seq: 0,
        }
    }
}

impl Actor for Sequencer {
    type Context = actix::Context<Self>;
}

impl Handler<EnclaveEvent<Unsequenced>> for Sequencer {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent<Unsequenced>, _: &mut Self::Context) -> Self::Result {
        // NOTE: FAKE SEQUENCER FOR NOW - JUST SET THE SEQUENCE NUMBER AND UPDATE
        self.seq += 1;
        self.bus.do_send(msg.into_sequenced(self.seq))
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
