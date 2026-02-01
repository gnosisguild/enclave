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
    event_context::EventContext,
    hlc::Hlc,
    sequencer::Sequencer,
    traits::{
        ErrorDispatcher, ErrorFactory, EventConstructorWithTimestamp, EventFactory, EventPublisher,
        EventSubscriber,
    },
    EType, EnclaveEvent, EnclaveEventData, ErrorEvent, EventBus, EventContextManager, EventType,
    HistoryCollector, Sequenced, Subscribe, Unsequenced, Unsubscribe,
};

#[derive(Clone, Derivative)]
#[derivative(Debug, PartialEq, Eq)]
pub struct BusHandle {
    /// EventBus that actors can consume sequenced events from
    event_bus: Addr<EventBus<EnclaveEvent<Sequenced>>>,
    /// Sequencer that new events should be produced from
    sequencer: Addr<Sequencer>,
    /// Hlc clock used to time all events created on this BusHandle
    #[derivative(Debug = "ignore")]
    hlc: Arc<Hlc>,
    /// Temporary context for events the bus publishes
    ctx: Option<EventContext<Sequenced>>,
}

impl BusHandle {
    /// Create a new BusHandle
    pub fn new(
        event_bus: Addr<EventBus<EnclaveEvent<Sequenced>>>,
        sequencer: Addr<Sequencer>,
        hlc: Hlc,
    ) -> Self {
        Self {
            event_bus,
            sequencer,
            hlc: Arc::new(hlc),
            ctx: None,
        }
    }

    /// Return a HistoryCollector for examining events that have passed through on the events bus
    pub fn history(&self) -> Addr<HistoryCollector<EnclaveEvent<Sequenced>>> {
        EventBus::<EnclaveEvent<Sequenced>>::history(&self.event_bus)
    }

    /// Access the sequencer to internally dispatch an event to
    pub fn sequencer(&self) -> &Addr<Sequencer> {
        &self.sequencer
    }

    /// Access the event_bus to internally subscribe to events
    pub fn event_bus(&self) -> &Addr<EventBus<EnclaveEvent<Sequenced>>> {
        &self.event_bus
    }

    /// Get a new timestamp. Note this ticks over the internal Hlc.
    pub fn ts(&self) -> Result<u128> {
        let ts = self.hlc.tick()?;
        Ok(ts.into())
    }

    /// Pipe events from this handle to the other handle only when the predicate returns true
    pub fn pipe_to<F>(&self, other: &BusHandle, predicate: F)
    where
        F: Fn(&EnclaveEvent<Sequenced>) -> bool + Unpin + 'static,
    {
        let pipe = BusHandlePipe::new(other.to_owned(), predicate).start();
        self.subscribe(EventType::All, pipe.into());
    }

    pub fn with_ec(&self, ec: &EventContext<Sequenced>) -> Self {
        let mut bus = self.clone();
        bus.set_ctx(ec.clone());
        bus
    }
}

impl EventPublisher<EnclaveEvent<Unsequenced>> for BusHandle {
    fn publish(
        &self,
        data: impl Into<EnclaveEventData>,
        ctx: EventContext<Sequenced>,
    ) -> Result<()> {
        self.publish_local(data, Some(ctx))
    }

    fn publish_without_context(&self, data: impl Into<EnclaveEventData>) -> Result<()> {
        self.publish_local(data, None)
    }

    fn publish_from_remote(
        &self,
        data: impl Into<EnclaveEventData>,
        remote_ts: u128,
    ) -> Result<()> {
        self.publish_from_remote_impl(data, remote_ts, None)
    }

    fn publish_from_remote_as_response(
        &self,
        data: impl Into<EnclaveEventData>,
        remote_ts: u128,
        caused_by: EventContext<Sequenced>,
    ) -> Result<()> {
        self.publish_from_remote_impl(data, remote_ts, Some(caused_by))
    }

    fn naked_dispatch(&self, event: EnclaveEvent<Unsequenced>) {
        self.sequencer.do_send(event);
    }
}

impl BusHandle {
    fn publish_from_remote_impl(
        &self,
        data: impl Into<EnclaveEventData>,
        remote_ts: u128,
        caused_by: Option<EventContext<Sequenced>>,
    ) -> Result<()> {
        let evt = self.event_from_remote_source(data, caused_by, remote_ts)?;
        self.sequencer.do_send(evt);
        Ok(())
    }
    fn publish_local(
        &self,
        data: impl Into<EnclaveEventData>,
        ctx: Option<EventContext<Sequenced>>,
    ) -> Result<()> {
        let evt = self.event_from(data, ctx)?;
        self.sequencer.do_send(evt);
        Ok(())
    }
}

impl ErrorDispatcher<EnclaveEvent<Unsequenced>> for BusHandle {
    fn err(&self, err_type: EType, error: impl Into<anyhow::Error>) {
        match self.event_from_error(err_type, error, self.get_ctx()) {
            Ok(evt) => self.sequencer.do_send(evt),
            Err(e) => error!("{e}"),
        }
    }
}

impl EventFactory<EnclaveEvent<Unsequenced>> for BusHandle {
    fn event_from(
        &self,
        data: impl Into<EnclaveEventData>,
        caused_by: Option<EventContext<Sequenced>>,
    ) -> Result<EnclaveEvent<Unsequenced>> {
        let ts = self.hlc.tick()?;
        Ok(EnclaveEvent::<Unsequenced>::new_with_timestamp(
            data.into(),
            caused_by,
            ts.into(),
        ))
    }

    fn event_from_remote_source(
        &self,
        data: impl Into<EnclaveEventData>,
        caused_by: Option<EventContext<Sequenced>>,
        ts: u128,
    ) -> Result<EnclaveEvent<Unsequenced>> {
        let ts = self.hlc.receive(&ts.into())?;
        Ok(EnclaveEvent::<Unsequenced>::new_with_timestamp(
            data.into(),
            caused_by,
            ts.into(),
        ))
    }
}

impl ErrorFactory<EnclaveEvent<Unsequenced>> for BusHandle {
    fn event_from_error(
        &self,
        err_type: EType,
        error: impl Into<anyhow::Error>,
        caused_by: Option<EventContext<Sequenced>>,
    ) -> Result<EnclaveEvent<Unsequenced>> {
        let ts = self.hlc.tick()?;
        EnclaveEvent::<Unsequenced>::from_error(err_type, error, ts.into(), caused_by)
    }
}

impl EventSubscriber<EnclaveEvent<Sequenced>> for BusHandle {
    fn subscribe(&self, event_type: EventType, recipient: Recipient<EnclaveEvent<Sequenced>>) {
        self.event_bus
            .do_send(Subscribe::new(event_type, recipient))
    }

    fn subscribe_all(
        &self,
        event_types: &[EventType],
        recipient: Recipient<EnclaveEvent<Sequenced>>,
    ) {
        for event_type in event_types.into_iter() {
            self.event_bus
                .do_send(Subscribe::new(*event_type, recipient.clone()));
        }
    }

    fn unsubscribe(&self, event_type: &str, recipient: Recipient<EnclaveEvent<Sequenced>>) {
        self.event_bus
            .do_send(Unsubscribe::new(event_type, recipient));
    }
}

impl EventContextManager for BusHandle {
    fn set_ctx<C>(&mut self, value: C)
    where
        C: Into<EventContext<Sequenced>>,
    {
        self.ctx = Some(value.into().clone());
    }
    fn get_ctx(&self) -> Option<EventContext<Sequenced>> {
        self.ctx.clone()
    }
}

#[cfg(test)]
mod tests {
    use actix::{Actor, Handler, Message};
    use e3_ciphernode_builder::EventSystem;
    // NOTE: We cannot pull from crate as the features will be missing as they are not default.
    use e3_events::{
        hlc::Hlc, prelude::*, BusHandle, EnclaveEvent, EnclaveEventData, EventPublisher, EventType,
        TestEvent,
    };
    use std::time::{Duration, SystemTime, UNIX_EPOCH};
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
                let ts = msg.ts();
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
        bus_a.subscribe(EventType::All, forwarder.clone().into());
        bus_b.subscribe(EventType::All, forwarder.into());

        // Create and subscribe the Saver to bus_c
        let saver = Saver { events: vec![] }.start();
        bus_c.subscribe(EventType::All, saver.clone().into());

        // Publish events in causal order across buses
        bus_a.publish_without_context(TestEvent::new("one", 1))?;
        sleep(Duration::from_millis(5)).await; // next tick
        bus_b.publish_without_context(TestEvent::new("two", 2))?;
        sleep(Duration::from_millis(5)).await; // next tick
        bus_a.publish_without_context(TestEvent::new("three", 3))?;
        sleep(Duration::from_millis(5)).await; // next tick
        bus_b.publish_without_context(TestEvent::new("four", 4))?;
        sleep(Duration::from_millis(50)).await; // next tick

        // Get events
        let events = saver.send(GetEventsOrdered).await?;

        // Sort by HLC timestamp
        let mut sorted_events = events.clone();
        sorted_events.sort_by_key(|e| e.ts());

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
        let timestamps: Vec<_> = sorted_events.iter().map(|e| e.ts()).collect();
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
    /// Create a new BusHandlePipe only forwarding events to the wrapped handle when the predicate
    /// function returns true
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
    fn handle(&mut self, msg: EnclaveEvent<Sequenced>, _: &mut Self::Context) -> Self::Result {
        if (self.predicate)(&msg) {
            let (data, ts) = msg.split();
            let _ = self.handle.publish_from_remote(data, ts);
        }
    }
}
