// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::sync::Arc;

use actix::{Actor, Addr, Handler, Recipient};
use anyhow::Result;
use derivative::Derivative;
use tracing::error;

use crate::{
    hlc::Hlc,
    sequencer::Sequencer,
    traits::{
        ErrorDispatcher, ErrorFactory, EventConstructorWithTimestamp, EventFactory, EventPublisher,
        EventSubscriber,
    },
    EType, EnclaveEvent, EnclaveEventData, ErrorEvent, EventBus, HistoryCollector, Sequenced,
    Subscribe, Unsequenced,
};

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq)]
pub struct BusHandle {
    /// EventBus that actors can consume sequenced events from
    consumer: Addr<EventBus<EnclaveEvent<Sequenced>>>,
    /// Sequencer that new events should be produced from
    producer: Addr<Sequencer>,
    /// Hlc clock used to time all events created on this BusHandle
    #[derivative(Debug = "ignore")]
    hlc: Arc<Hlc>,
}

impl BusHandle {
    /// Constructs a BusHandle that connects an EventBus consumer with a Sequencer producer and an HLC clock.
    ///
    /// The provided HLC is associated with the handle and used to timestamp events created by it.
    ///
    /// # Examples
    ///
    /// ```
    /// // Given existing `consumer: Addr<EventBus<EnclaveEvent<Sequenced>>>`,
    /// // `producer: Addr<Sequencer>` and `hlc: Hlc`:
    /// let handle = BusHandle::new(consumer, producer, hlc);
    /// // `handle` can now be used to publish and subscribe to events.
    /// ```
    pub fn new(
        consumer: Addr<EventBus<EnclaveEvent<Sequenced>>>,
        producer: Addr<Sequencer>,
        hlc: Hlc,
    ) -> Self {
        Self {
            consumer,
            producer,
            hlc: Arc::new(hlc),
        }
    }

    /// Returns the HistoryCollector actor address for this bus's sequenced events.
    ///
    /// The returned address can be used to query or inspect events that have passed through the consumer EventBus.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use actix::prelude::*;
    /// # use crates::events::{BusHandle, EnclaveEvent, Sequenced, HistoryCollector};
    /// # fn example(handle: &BusHandle) {
    /// let hist: Addr<HistoryCollector<EnclaveEvent<Sequenced>>> = handle.history();
    /// # }
    /// ```
    pub fn history(&self) -> Addr<HistoryCollector<EnclaveEvent<Sequenced>>> {
        EventBus::<EnclaveEvent<Sequenced>>::history(&self.consumer)
    }

    /// Access the handle's internal producer actor.
    ///
    /// # Returns
    ///
    /// A reference to the internal producer address (`Addr<Sequencer>`).
    ///
    /// # Examples
    ///
    /// ```
    /// // `consumer_addr`, `producer_addr`, and `hlc` are assumed to be available.
    /// let handle = BusHandle::new(consumer_addr, producer_addr, hlc);
    /// let producer_ref = handle.producer();
    /// // `producer_ref` can be used to send messages to the producer actor.
    /// ```
    pub fn producer(&self) -> &Addr<Sequencer> {
        &self.producer
    }

    /// Access the consumer to internally subscribe to events
    pub fn consumer(&self) -> &Addr<EventBus<EnclaveEvent<Sequenced>>> {
        &self.consumer
    }

    /// Produces a new timestamp from the handle's HLC and advances the internal clock.
    ///
    /// # Returns
    ///
    /// `Ok(u128)` containing the new HLC-derived timestamp, or an `Err` if advancing the HLC fails.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assume `handle` is a BusHandle available in scope.
    /// let ts = handle.ts().expect("failed to obtain timestamp");
    /// println!("timestamp = {}", ts);
    /// ```
    pub fn ts(&self) -> Result<u128> {
        let ts = self.hlc.tick()?;
        Ok(ts.into())
    }

    /// Create a BusHandlePipe actor that forwards matching sequenced events from this handle to another.
    ///
    /// Starts a BusHandlePipe actor and subscribes it to all event types ("*"). For each incoming
    /// `EnclaveEvent<Sequenced>`, the provided `predicate` is invoked; when it returns `true` the event
    /// is forwarded to the `other` handle with its original timestamp preserved.
    ///
    /// # Parameters
    ///
    /// - `other`: target `BusHandle` to forward matching events to.
    /// - `predicate`: function called for each `EnclaveEvent<Sequenced>`; return `true` to forward the event.
    ///
    /// # Examples
    ///
    /// ```
    /// // forward-only-important is an example predicate that forwards events whose data equals "important"
    /// let a: BusHandle = /* source handle */;
    /// let b: BusHandle = /* target handle */;
    /// a.pipe_to(&b, |ev: &EnclaveEvent<Sequenced>| ev.data == "important");
    /// ```
    pub fn pipe_to<F>(&self, other: &BusHandle, predicate: F)
    where
        F: Fn(&EnclaveEvent<Sequenced>) -> bool + Unpin + 'static,
    {
        let pipe = BusHandlePipe::new(other.to_owned(), predicate).start();
        self.subscribe("*", pipe.into());
    }
}

impl EventPublisher<EnclaveEvent<Unsequenced>> for BusHandle {
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

    fn naked_dispatch(&self, event: EnclaveEvent<Unsequenced>) {
        self.producer.do_send(event);
    }
}

impl ErrorDispatcher<EnclaveEvent<Unsequenced>> for BusHandle {
    fn err(&self, err_type: EType, error: impl Into<anyhow::Error>) {
        match self.event_from_error(err_type, error) {
            Ok(evt) => self.producer.do_send(evt),
            Err(e) => error!("{e}"),
        }
    }
}

impl EventFactory<EnclaveEvent<Unsequenced>> for BusHandle {
    fn event_from(&self, data: impl Into<EnclaveEventData>) -> Result<EnclaveEvent<Unsequenced>> {
        let ts = self.hlc.tick()?;
        Ok(EnclaveEvent::<Unsequenced>::new_with_timestamp(
            data.into(),
            ts.into(),
        ))
    }

    fn event_from_remote_source(
        &self,
        data: impl Into<EnclaveEventData>,
        ts: u128,
    ) -> Result<EnclaveEvent<Unsequenced>> {
        let ts = self.hlc.receive(&ts.into())?;
        Ok(EnclaveEvent::<Unsequenced>::new_with_timestamp(
            data.into(),
            ts.into(),
        ))
    }
}

impl ErrorFactory<EnclaveEvent<Unsequenced>> for BusHandle {
    fn event_from_error(
        &self,
        err_type: EType,
        error: impl Into<anyhow::Error>,
    ) -> Result<EnclaveEvent<Unsequenced>> {
        let ts = self.hlc.tick()?;
        EnclaveEvent::<Unsequenced>::from_error(err_type, error, ts.into())
    }
}

impl EventSubscriber<EnclaveEvent<Sequenced>> for BusHandle {
    fn subscribe(&self, event_type: &str, recipient: Recipient<EnclaveEvent<Sequenced>>) {
        self.consumer.do_send(Subscribe::new(event_type, recipient))
    }

    /// Subscribe a recipient to multiple event types on the internal consumer.
    ///
    /// Each provided event type will be registered with the consumer so the given recipient will receive events of those types. The recipient is cloned for each registration.
    ///
    /// # Examples
    ///
    /// ```
    /// use actix::prelude::*;
    /// // assume `handle` is a BusHandle and `recipient` implements `Recipient<EnclaveEvent<Sequenced>>`
    /// let types = vec!["type_a", "type_b"];
    /// handle.subscribe_all(&types, recipient);
    /// ```
    fn subscribe_all(&self, event_types: &[&str], recipient: Recipient<EnclaveEvent<Sequenced>>) {
        for event_type in event_types.into_iter() {
            self.consumer
                .do_send(Subscribe::new(*event_type, recipient.clone()));
        }
    }
}

#[cfg(test)]
mod tests {
    use actix::{Actor, Handler, Message};
    use e3_ciphernode_builder::EventSystem;
    // NOTE: We cannot pull from crate as the features will be missing as they are not default.
    use e3_events::{
        hlc::Hlc, prelude::*, BusHandle, EnclaveEvent, EnclaveEventData, EventPublisher, TestEvent,
    };
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
    use tokio::time::sleep;

    /// Get the current system time as microseconds since the UNIX epoch.
    ///
    /// Returns the number of microseconds elapsed since 1970-01-01 00:00:00 UTC.
    ///
    /// # Examples
    ///
    /// ```
    /// let t = now_micros();
    /// assert!(t > 0);
    /// ```
    fn now_micros() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as u64
    }

    /// Verifies that HLC timestamps preserve causal ordering and monotonicity across clock-drifted buses.
    ///
    /// Sets up three EventSystem buses (A, B, C) where A's clock is 30 seconds behind B's, forwards all
    /// events from A and B to C, publishes four causally-ordered test events across A and B, and then
    /// collects the events observed on C. Asserts that:
    /// - the causal order of messages ("one","two","three","four") is preserved when sorted by HLC timestamp,
    /// - all HLC timestamps are unique,
    /// - timestamps are strictly increasing when ordered.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// // Run the test suite; this test will execute as part of `cargo test`.
    /// // cargo test --test crates_events
    /// ```
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
        let bus_a = EventSystem::new("a")
            .with_fresh_bus()
            .with_hlc(Hlc::new(1).with_clock(move || now_micros().saturating_sub(30_000_000))) // Late
            .handle()?;
        let bus_b = EventSystem::new("b")
            .with_fresh_bus()
            .with_hlc(Hlc::new(2))
            .handle()?;
        let bus_c = EventSystem::new("c")
            .with_fresh_bus()
            .with_hlc(Hlc::new(3))
            .handle()?;

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

/// Actor for piping between BusHandles.
pub struct BusHandlePipe<F>
where
    F: Fn(&EnclaveEvent<Sequenced>) -> bool + Unpin + 'static,
{
    handle: BusHandle,
    predicate: F,
}

impl<F> BusHandlePipe<F>
where
    F: Fn(&EnclaveEvent<Sequenced>) -> bool + Unpin + 'static,
{
    /// Creates a BusHandlePipe that forwards events to the provided `BusHandle` only when the predicate returns `true`.
    ///
    /// # Parameters
    ///
    /// - `handle`: The destination `BusHandle` to which matching events will be forwarded.
    /// - `predicate`: A function that receives an `EnclaveEvent<Sequenced>` and returns `true` to forward the event or `false` to drop it.
    ///
    /// # Examples
    ///
    /// ```
    /// // Assume `handle` is a valid BusHandle and `event` is an EnclaveEvent<Sequenced>.
    /// let pipe = BusHandlePipe::new(handle, |evt: &EnclaveEvent<Sequenced>| evt.data().len() > 0);
    /// // The pipe will forward only events whose data length is greater than zero.
    /// ```
    pub fn new(handle: BusHandle, predicate: F) -> Self {
        Self { handle, predicate }
    }
}

impl<F> Actor for BusHandlePipe<F>
where
    F: Fn(&EnclaveEvent<Sequenced>) -> bool + Unpin + 'static,
{
    type Context = actix::Context<Self>;
}

impl<F> Handler<EnclaveEvent<Sequenced>> for BusHandlePipe<F>
where
    F: Fn(&EnclaveEvent<Sequenced>) -> bool + Unpin + 'static,
{
    type Result = ();
    /// Forwards an incoming `EnclaveEvent<Sequenced>` to the wrapped `BusHandle` when the pipe's predicate returns `true`.
    ///
    /// If the predicate accepts the event, the event is split into its data and timestamp and forwarded via
    /// `publish_from_remote` on the inner handle. If the predicate rejects the event, the message is ignored.
    ///
    /// # Examples
    ///
    /// ```
    /// // Placeholder example — replace `MyHandle`, `EnclaveEvent`, and context creation with concrete types from the crate.
    /// # use std::sync::Arc;
    /// # use actix::prelude::*;
    /// // let handle: MyHandle = /* existing BusHandle */ unimplemented!();
    /// // let mut pipe = BusHandlePipe::new(handle, |ev: &EnclaveEvent<Sequenced>| true);
    /// // let mut ctx = Context::new(&mut pipe);
    /// // let event: EnclaveEvent<Sequenced> = /* create or receive an event */ unimplemented!();
    /// // pipe.handle(event, &mut ctx);
    /// ```
    fn handle(&mut self, msg: EnclaveEvent<Sequenced>, _: &mut Self::Context) -> Self::Result {
        if (self.predicate)(&msg) {
            let (data, ts) = msg.split();
            let _ = self.handle.publish_from_remote(data, ts);
        }
    }
}