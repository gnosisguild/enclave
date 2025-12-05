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
    pub fn new_from_consumer(consumer: Addr<EventBus<EnclaveEvent<Stored>>>) -> Self {
        let producer = Sequencer::new(&consumer).start();
        let hlc = Hlc::default();
        Self::new(consumer, producer, hlc)
    }

    pub fn new(
        consumer: Addr<EventBus<EnclaveEvent<Stored>>>,
        producer: Addr<Sequencer>,
        hlc: Hlc,
    ) -> Self {
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
        BusHandle::new_from_consumer(self)
    }
}

impl Into<BusHandle> for &Addr<EventBus<EnclaveEvent>> {
    fn into(self) -> BusHandle {
        BusHandle::new_from_consumer(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use crate::{
        hlc::Hlc, prelude::*, sequencer::Sequencer, BusHandle, EnclaveEvent, EnclaveEventData,
        EventBus, TestEvent,
    };
    use actix::{Actor, Handler, Message};
    use tokio::time::sleep;
    fn now_micros() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64
    }

    #[actix::test]
    async fn test_hlc_events() -> anyhow::Result<()> {
        #[derive(Message)]
        #[rtype("Vec<EnclaveEvent>")]
        struct GetEventsOrdered;

        // Setup forwarder
        struct Forwarder {
            dest: BusHandle,
        }
        impl Actor for Forwarder {
            type Context = actix::Context<Self>;
        }

        impl Handler<EnclaveEvent> for Forwarder {
            type Result = ();
            fn handle(&mut self, msg: EnclaveEvent, _: &mut Self::Context) -> Self::Result {
                let ts = msg.get_ts();
                self.dest.publish_from_remote(msg.into_data(), ts).unwrap()
            }
        }

        // Setup saver
        struct Saver {
            events: Vec<EnclaveEvent>,
        }

        impl Actor for Saver {
            type Context = actix::Context<Self>;
        }

        impl Handler<EnclaveEvent> for Saver {
            type Result = ();
            fn handle(&mut self, msg: EnclaveEvent, _: &mut Self::Context) -> Self::Result {
                self.events.push(msg);
            }
        }

        impl Handler<GetEventsOrdered> for Saver {
            type Result = Vec<EnclaveEvent>;
            fn handle(&mut self, _: GetEventsOrdered, _: &mut Self::Context) -> Self::Result {
                self.events.clone()
            }
        }

        // 1. setup up two separate busses with out of sync clocks A and B. B should be 30 seconds
        //    faster than A.
        let consumer_a = EventBus::<EnclaveEvent>::default().start();
        let producer_a = Sequencer::new(&consumer_a).start();
        let clock_a = Hlc::new(1).with_clock(move || now_micros().saturating_sub(30_000_000)); // Late
        let bus_a = BusHandle::new(consumer_a, producer_a, clock_a);

        let consumer_b = EventBus::<EnclaveEvent>::default().start();
        let producer_b = Sequencer::new(&consumer_b).start();
        let clock_b = Hlc::new(2); // in sync
        let bus_b = BusHandle::new(consumer_b, producer_b, clock_b);

        let consumer_c = EventBus::<EnclaveEvent>::default().start();
        let producer_c = Sequencer::new(&consumer_c).start();
        let clock_c = Hlc::new(3); // in sync
        let bus_c = BusHandle::new(consumer_c, producer_c, clock_c);

        let forwarder = Forwarder {
            dest: bus_c.clone(),
        }
        .start();

        // pipe all bus_a and bus_b events to bus_c
        bus_a.subscribe("*", forwarder.clone().into());
        bus_b.subscribe("*", forwarder.into());

        // Create and subscribe the Saver to bus_c
        let saver = Saver { events: vec![] }.start();
        bus_c.subscribe("*", saver.clone().into());

        // Publish events in causal order across buses
        bus_a.publish(TestEvent::new("one", 1))?;
        sleep(Duration::from_millis(5)).await; // next tick
        bus_b.publish(TestEvent::new("two", 2))?;
        sleep(Duration::from_millis(5)).await; // next tick
        bus_a.publish(TestEvent::new("three", 3))?;
        sleep(Duration::from_millis(5)).await; // next tick
        bus_b.publish(TestEvent::new("four", 4))?;
        sleep(Duration::from_millis(50)).await; // next tick

        // Get events
        let events = saver.send(GetEventsOrdered).await?;

        // Sort by HLC timestamp
        let mut sorted_events = events.clone();
        sorted_events.sort_by_key(|e| e.get_ts());

        // Extract the payloads/names in HLC-sorted order
        let ordered_names: Vec<_> = sorted_events
            .iter()
            .filter_map(|e| match e.get_data() {
                EnclaveEventData::TestEvent(e) => Some(e.msg.clone()),
                _ => None,
            })
            .collect();

        // ASSERTION 1: Causal order is preserved despite clock drift
        assert_eq!(
            ordered_names,
            vec!["one", "two", "three", "four"],
            "HLC should preserve causal ordering despite 30s clock drift on bus_a"
        );

        // ASSERTION 2: All timestamps are unique (HLC guarantee)
        let timestamps: Vec<_> = sorted_events.iter().map(|e| e.get_ts()).collect();
        let unique_timestamps: std::collections::HashSet<_> = timestamps.iter().collect();
        assert_eq!(
            timestamps.len(),
            unique_timestamps.len(),
            "All HLC timestamps should be unique"
        );

        // ASSERTION 3: Timestamps are strictly monotonically increasing when sorted
        for window in timestamps.windows(2) {
            assert!(
                window[0] < window[1],
                "HLC timestamps should be strictly increasing: {:?} should be < {:?}",
                window[0],
                window[1]
            );
        }

        Ok(())
    }
}
