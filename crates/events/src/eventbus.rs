// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::prelude::*;
use bloom::{BloomFilter, ASMS};
use std::collections::HashMap;
use std::fmt::Display;
use std::hash::Hash;
use std::marker::PhantomData;
use tokio::sync::oneshot;

use crate::EnclaveEvent;

//////////////////////////////////////////////////////////////////////////////
// Core Traits
//////////////////////////////////////////////////////////////////////////////

/// Trait that must be implemented by events used with EventBus
pub trait Event: Message<Result = ()> + Clone + Display + Send + Sync + Unpin + 'static {
    type Id: Hash + Eq + Clone + Unpin + Send + Sync;
    fn event_type(&self) -> String;
    fn event_id(&self) -> Self::Id;
}

/// Trait for events that contain an error
pub trait ErrorEvent: Event {
    type Error: Clone;
    type ErrorType;

    fn as_error(&self) -> Option<&Self::Error>;
    fn from_error(err_type: Self::ErrorType, error: anyhow::Error) -> Self;
}

//////////////////////////////////////////////////////////////////////////////
// Configuration
//////////////////////////////////////////////////////////////////////////////

/// Configuration for EventBus behavior
pub struct EventBusConfig {
    pub deduplicate: bool,
}

impl Default for EventBusConfig {
    fn default() -> Self {
        Self { deduplicate: true }
    }
}

fn default_bloomfilter() -> BloomFilter {
    let num_items = 10000000;
    let fp_rate = 0.001;
    BloomFilter::with_rate(fp_rate, num_items)
}

//////////////////////////////////////////////////////////////////////////////
// EventBus Implementation
//////////////////////////////////////////////////////////////////////////////
/// Central EventBus for each node. Actors publish events to this bus by sending it EnclaveEvents.
/// All events sent to this bus are assumed to be published over the network via pubsub.
/// Other actors such as the NetEventTranslator and Evm actor connect to outside services and control which events
/// actually get published as well as ensure that local events are not rebroadcast locally after
/// being published.
pub struct EventBus<E: Event> {
    config: EventBusConfig,
    ids: BloomFilter,
    listeners: HashMap<String, Vec<Recipient<E>>>,
}

impl<E: Event> Actor for EventBus<E> {
    type Context = Context<Self>;
}

impl<E: Event> EventBus<E> {
    pub fn new(config: EventBusConfig) -> Self {
        EventBus {
            config,
            listeners: HashMap::new(),
            ids: default_bloomfilter(),
        }
    }

    pub fn set_config(&mut self, config: EventBusConfig) {
        self.config = config;
    }

    fn track(&mut self, event: E) {
        self.ids.insert(&event.event_id());
    }

    fn is_duplicate(&self, event: &E) -> bool {
        self.ids.contains(&event.event_id())
    }
}

impl<E: Event> Default for EventBus<E> {
    fn default() -> Self {
        Self {
            config: EventBusConfig::default(),
            listeners: HashMap::new(),
            ids: default_bloomfilter(),
        }
    }
}

impl<E: Event> Handler<E> for EventBus<E> {
    type Result = ();

    fn handle(&mut self, event: E, _: &mut Context<Self>) {
        if self.is_duplicate(&event) {
            return;
        }

        if let Some(listeners) = self.listeners.get("*") {
            for listener in listeners {
                listener.do_send(event.clone())
            }
        }

        if let Some(listeners) = self.listeners.get(&event.event_type()) {
            for listener in listeners {
                listener.do_send(event.clone())
            }
        }

        // TODO: workshop to work out best display format
        tracing::info!(">>> {}", event);

        self.track(event);
    }
}

//////////////////////////////////////////////////////////////////////////////
// Subscribe Message
//////////////////////////////////////////////////////////////////////////////

#[derive(Message)]
#[rtype(result = "()")]
pub struct Subscribe<E: Event> {
    pub event_type: String,
    pub listener: Recipient<E>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Unsubscribe<E: Event> {
    pub event_type: String,
    pub listener: Recipient<E>,
}

impl<E: Event> Subscribe<E> {
    pub fn new(event_type: impl Into<String>, listener: Recipient<E>) -> Self {
        Self {
            event_type: event_type.into(),
            listener,
        }
    }
}

impl<E: Event> Unsubscribe<E> {
    pub fn new(event_type: impl Into<String>, listener: Recipient<E>) -> Self {
        Self {
            event_type: event_type.into(),
            listener,
        }
    }
}

impl<E: Event> Handler<Subscribe<E>> for EventBus<E> {
    type Result = ();

    fn handle(&mut self, msg: Subscribe<E>, _: &mut Context<Self>) {
        self.listeners
            .entry(msg.event_type)
            .or_default()
            .push(msg.listener);
    }
}

impl<E: Event> Handler<Unsubscribe<E>> for EventBus<E> {
    type Result = ();

    fn handle(&mut self, msg: Unsubscribe<E>, _: &mut Context<Self>) {
        if let Some(listeners) = self.listeners.get_mut(&msg.event_type) {
            listeners.retain(|listener| listener != &msg.listener);
        }
    }
}

//////////////////////////////////////////////////////////////////////////////
// History Management
//////////////////////////////////////////////////////////////////////////////

#[derive(Message)]
#[rtype(result = "Vec<E>")]
pub struct GetHistory<E: Event>(PhantomData<E>);

impl<E: Event> GetHistory<E> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

impl<E: Event> Handler<GetHistory<E>> for HistoryCollector<E> {
    type Result = Vec<E>;

    fn handle(&mut self, _: GetHistory<E>, _: &mut Context<Self>) -> Vec<E> {
        self.history.clone()
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ResetHistory;

impl<E: Event> Handler<ResetHistory> for HistoryCollector<E> {
    type Result = ();

    fn handle(&mut self, _: ResetHistory, _: &mut Context<Self>) {
        self.history.clear()
    }
}

#[derive(Message)]
#[rtype(result = "Vec<E::Error>")]
pub struct GetErrors<E: ErrorEvent>(PhantomData<E>);

impl<E: ErrorEvent> GetErrors<E> {
    pub fn new() -> Self {
        Self(PhantomData)
    }
}

//////////////////////////////////////////////////////////////////////////////
// Error Bus Trait
//////////////////////////////////////////////////////////////////////////////

/// Trait to send errors directly to the bus
pub trait BusError<E: ErrorEvent> {
    fn err(&self, err_type: E::ErrorType, err: anyhow::Error);
}

impl<E: ErrorEvent> BusError<E> for Addr<EventBus<E>> {
    fn err(&self, err_type: E::ErrorType, err: anyhow::Error) {
        self.do_send(E::from_error(err_type, err))
    }
}

impl<E: ErrorEvent> BusError<E> for Recipient<E> {
    fn err(&self, err_type: E::ErrorType, err: anyhow::Error) {
        self.do_send(E::from_error(err_type, err))
    }
}

//////////////////////////////////////////////////////////////////////////////
// History Collector
//////////////////////////////////////////////////////////////////////////////

/// Actor to subscribe to EventBus to capture all history
pub struct HistoryCollector<E: Event> {
    history: Vec<E>,
}

impl<E: ErrorEvent> HistoryCollector<E> {
    pub fn new() -> Self {
        Self {
            history: Vec::new(),
        }
    }
}

impl<E: Event> Actor for HistoryCollector<E> {
    type Context = Context<Self>;
}

impl<E: Event> Handler<E> for HistoryCollector<E> {
    type Result = E::Result;
    fn handle(&mut self, msg: E, _ctx: &mut Self::Context) -> Self::Result {
        self.history.push(msg);
    }
}

//////////////////////////////////////////////////////////////////////////////
// EventWaiter
//////////////////////////////////////////////////////////////////////////////

/// Actor to wait on specific events in order to help with testing
pub struct EventWaiter<E>
where
    E: Event + Clone,
{
    tx: Option<oneshot::Sender<E>>,
    matcher: Box<dyn Fn(&E) -> bool + Send + 'static>,
    bus: Addr<EventBus<E>>,
}

impl<E> EventWaiter<E>
where
    E: Event + Clone,
{
    pub fn new(
        bus: Addr<EventBus<E>>,
        tx: oneshot::Sender<E>,
        matcher: Box<dyn Fn(&E) -> bool + Send + 'static>,
    ) -> Self {
        Self {
            tx: Some(tx),
            matcher,
            bus,
        }
    }

    pub fn wait(
        bus: &Addr<EventBus<E>>,
        matcher: Box<dyn Fn(&E) -> bool + Send + 'static>,
    ) -> oneshot::Receiver<E> {
        let (tx, rx) = oneshot::channel::<E>();
        let addr = Self::new(bus.clone(), tx, matcher).start();
        bus.do_send(Subscribe::new("*", addr.recipient()));
        rx
    }
}

impl<E> Actor for EventWaiter<E>
where
    E: Event + Clone,
{
    type Context = Context<Self>;
}

impl<E> Handler<E> for EventWaiter<E>
where
    E: Event + Clone,
{
    type Result = ();

    fn handle(&mut self, msg: E, ctx: &mut Self::Context) -> Self::Result {
        if (self.matcher)(&msg) {
            if let Some(tx) = self.tx.take() {
                let _ = tx.send(msg.clone());
                self.bus.do_send(Unsubscribe::new(
                    msg.event_type(),
                    ctx.address().recipient(),
                ));
                ctx.stop();
            }
        }
    }
}

/// Prepare a receiver to return the first event that passes the matcher function from the event
/// bus. You must return the receiver first before triggering any events.
pub fn wait_for_event<E>(
    bus: &Addr<EventBus<E>>,
    matcher: Box<dyn Fn(&E) -> bool + Send + 'static>,
) -> oneshot::Receiver<E>
where
    E: Event + Clone,
{
    EventWaiter::wait(bus, matcher)
}

//////////////////////////////////////////////////////////////////////////////
// Error Collector
//////////////////////////////////////////////////////////////////////////////

/// Actor to subscribe to EventBus to capture errors
pub struct ErrorCollector<E: ErrorEvent> {
    errors: Vec<E>,
}

impl<E: ErrorEvent> ErrorCollector<E> {
    pub fn new() -> Self {
        Self { errors: Vec::new() }
    }
}

impl<E: ErrorEvent> Actor for ErrorCollector<E> {
    type Context = Context<Self>;
}

impl<E: ErrorEvent> Handler<E> for ErrorCollector<E> {
    type Result = E::Result;
    fn handle(&mut self, msg: E, _: &mut Self::Context) -> Self::Result {
        if let Some(_) = msg.as_error() {
            self.errors.push(msg);
        }
    }
}

impl<E: ErrorEvent> Handler<GetErrors<E>> for ErrorCollector<E> {
    type Result = Vec<E::Error>;

    fn handle(&mut self, _: GetErrors<E>, _: &mut Context<Self>) -> Vec<E::Error> {
        self.errors
            .iter()
            .filter_map(|evt| evt.as_error())
            .cloned()
            .collect()
    }
}

//////////////////////////////////////////////////////////////////////////////
// Test Helper Functions
//////////////////////////////////////////////////////////////////////////////

/// Function to help with testing when we want to maintain a vec of events
pub fn new_event_bus_with_history<E: Event>() -> (Addr<EventBus<E>>, Addr<HistoryCollector<E>>) {
    let bus = EventBus::<E>::default().start();

    let history = HistoryCollector {
        history: Vec::new(),
    }
    .start();

    bus.do_send(Subscribe::new("*", history.clone().recipient()));
    (bus, history)
}
