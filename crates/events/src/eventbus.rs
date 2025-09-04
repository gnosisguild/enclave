// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::prelude::*;
use anyhow::{anyhow, Result};
use bloom::{BloomFilter, ASMS};
use std::collections::{HashMap, VecDeque};
use std::fmt::Display;
use std::hash::Hash;
use std::marker::PhantomData;
use tokio::sync::oneshot;

//////////////////////////////////////////////////////////////////////////////
// Core Traits
//////////////////////////////////////////////////////////////////////////////

/// Trait that must be implemented by events used with EventBus
pub trait Event: Message<Result = ()> + Clone + Display + Send + Sync + Unpin + 'static {
    type Id: Hash + Eq + Clone + Unpin + Send + Sync + Display;
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

    pub fn history(source: &Addr<EventBus<E>>) -> Addr<HistoryCollector<E>> {
        let addr = HistoryCollector::<E>::new().start();
        source.do_send(Subscribe::new("*", addr.clone().recipient()));
        addr
    }
    pub fn error<EE: ErrorEvent>(source: &Addr<EventBus<EE>>) -> Addr<ErrorCollector<EE>> {
        let addr = ErrorCollector::<EE>::new().start();
        source.do_send(Subscribe::new("*", addr.clone().recipient()));
        addr
    }

    pub fn pipe(source: &Addr<EventBus<E>>, dest: &Addr<EventBus<E>>) {
        source.do_send(Subscribe::new("*", dest.clone().recipient()))
    }

    pub fn pipe_filter<F>(source: &Addr<EventBus<E>>, predicate: F, dest: &Addr<EventBus<E>>)
    where
        F: Fn(&E) -> bool + 'static,
    {
        let filter = EventFilter::new(dest.clone().recipient(), predicate).start();

        source.do_send(Subscribe::new("*", filter.recipient()));
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
            println!(
                "IS DUPLICATE: {}:{}:{}",
                event.event_type(),
                event.event_id(),
                event
            );
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
// Event Filter
//////////////////////////////////////////////////////////////////////////////

pub type Predicate<E> = Box<dyn Fn(&E) -> bool>;

pub struct EventFilter<E: Event> {
    dest: Recipient<E>,
    predicate: Predicate<E>,
}

impl<E: Event> EventFilter<E> {
    pub fn new<F>(dest: Recipient<E>, predicate: F) -> Self
    where
        F: Fn(&E) -> bool + 'static,
    {
        Self {
            dest,
            predicate: Box::new(predicate),
        }
    }
}

impl<E: Event> Actor for EventFilter<E> {
    type Context = actix::Context<Self>;
}

impl<E: Event> Handler<E> for EventFilter<E> {
    type Result = ();
    fn handle(&mut self, msg: E, _: &mut Self::Context) -> Self::Result {
        if (self.predicate)(&msg) {
            self.dest.do_send(msg);
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

#[derive(Message)]
#[rtype(result = "Vec<E>")]
pub struct TakeHistory<E: Event> {
    amount: usize,
    _d: PhantomData<E>,
}

impl<E: Event> TakeHistory<E> {
    pub fn new(amount: usize) -> Self {
        Self {
            amount,
            _d: PhantomData,
        }
    }
}

struct PendingTake<E: Event> {
    count: usize,
    collected: Vec<E>,
    responder: tokio::sync::oneshot::Sender<Vec<E>>,
}

impl<E: Event> Handler<GetHistory<E>> for HistoryCollector<E> {
    type Result = Vec<E>;

    fn handle(&mut self, _: GetHistory<E>, _: &mut Context<Self>) -> Vec<E> {
        println!("History gettin'...");
        self.history.iter().cloned().collect()
    }
}

impl<E: Event> Handler<TakeHistory<E>> for HistoryCollector<E> {
    type Result = ResponseActFuture<Self, Vec<E>>;

    fn handle(&mut self, msg: TakeHistory<E>, _: &mut Context<Self>) -> Self::Result {
        let count = msg.amount;

        // If we have enough events in history, return immediately
        if self.history.len() >= count {
            let events: Vec<E> = self.history.drain(..count).collect();
            return Box::pin(async move { events }.into_actor(self));
        }

        // Create a tokio oneshot channel for the response
        let (tx, rx) = tokio::sync::oneshot::channel();

        // Collect what we can from history
        let mut collected = Vec::new();
        while !self.history.is_empty() && collected.len() < count {
            collected.push(self.history.pop_front().unwrap());
        }

        // Store the pending request
        self.pending_takes.push(PendingTake {
            count,
            collected,
            responder: tx,
        });

        // Return future that waits for the response
        Box::pin(async move { rx.await.unwrap_or_else(|_| Vec::new()) }.into_actor(self))
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ResetHistory;

impl<E: Event> Handler<ResetHistory> for HistoryCollector<E> {
    type Result = ();

    fn handle(&mut self, _: ResetHistory, _: &mut Context<Self>) {
        println!("History clearn'");
        self.history.clear();
        self.pending_takes.clear();
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
    history: VecDeque<E>,
    pending_takes: Vec<PendingTake<E>>,
}

impl<E: Event> HistoryCollector<E> {
    pub fn new() -> Self {
        Self {
            history: VecDeque::new(),
            pending_takes: Vec::new(),
        }
    }

    fn try_fulfill_pending_takes(&mut self) {
        let mut completed = Vec::new();

        // For each pending take, try to fulfill it
        for (idx, pending) in self.pending_takes.iter_mut().enumerate() {
            // Fill from history first
            while pending.collected.len() < pending.count && !self.history.is_empty() {
                pending.collected.push(self.history.pop_front().unwrap());
            }

            // If we have enough, mark as complete
            if pending.collected.len() >= pending.count {
                completed.push(idx);
            }
        }

        // Send responses for completed takes (in reverse order to maintain indices)
        for idx in completed.into_iter().rev() {
            let pending = self.pending_takes.swap_remove(idx);
            let events = pending.collected.into_iter().take(pending.count).collect();
            let _ = pending.responder.send(events);
        }
    }
    fn add_event(&mut self, event: E) {
        // First try to give to pending takes
        for pending in &mut self.pending_takes {
            if pending.collected.len() < pending.count {
                pending.collected.push(event);
                self.try_fulfill_pending_takes();
                return;
            }
        }

        // No pending take needed it, add to history
        self.history.push_back(event);
    }
}

impl<E: Event> Actor for HistoryCollector<E> {
    type Context = Context<Self>;
}

impl<E: Event> Handler<E> for HistoryCollector<E> {
    type Result = E::Result;
    fn handle(&mut self, msg: E, _ctx: &mut Self::Context) -> Self::Result {
        println!(
            "History loggin' id={} type={}",
            msg.event_id(),
            msg.event_type()
        );
        self.add_event(msg);
    }
}

//////////////////////////////////////////////////////////////////////////////
// EventWaiter
//////////////////////////////////////////////////////////////////////////////
#[derive(Message)]
#[rtype(result = "()")]
struct TimeoutMessage;

/// Actor to wait on specific events in order to help with testing
pub struct EventWaiter<E>
where
    E: Event + Clone,
{
    tx: Option<oneshot::Sender<Result<E>>>,
    matcher: Box<dyn Fn(&E) -> bool + Send + 'static>,
    bus: Addr<EventBus<E>>,
}

impl<E> EventWaiter<E>
where
    E: Event + Clone,
{
    pub fn new(
        bus: Addr<EventBus<E>>,
        tx: oneshot::Sender<Result<E>>,
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
    ) -> oneshot::Receiver<Result<E>> {
        Self::wait_with_timeout(bus, matcher, 10000)
    }

    pub fn wait_with_timeout(
        bus: &Addr<EventBus<E>>,
        matcher: Box<dyn Fn(&E) -> bool + Send + 'static>,
        timeout: u64,
    ) -> oneshot::Receiver<Result<E>> {
        let (tx, rx) = oneshot::channel::<Result<E>>();
        let addr = Self::new(bus.clone(), tx, matcher).start();
        bus.do_send(Subscribe::new("*", addr.clone().recipient()));
        // Add timeout
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(timeout)).await;
            addr.do_send(TimeoutMessage);
        });
        rx
    }

    pub async fn send_and_wait(
        bus: &Addr<EventBus<E>>,
        event: E,
        matcher: Box<dyn Fn(&E) -> bool + Send + 'static>,
    ) -> Result<E> {
        let waiter = Self::wait(bus, matcher);
        bus.do_send(event);

        waiter.await?
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
                let _ = tx.send(Ok(msg.clone()));
                self.bus.do_send(Unsubscribe::new(
                    msg.event_type(),
                    ctx.address().recipient(),
                ));
                ctx.stop();
            }
        }
    }
}

impl<E> Handler<TimeoutMessage> for EventWaiter<E>
where
    E: Event + Clone,
{
    type Result = ();

    fn handle(&mut self, _msg: TimeoutMessage, ctx: &mut Self::Context) -> Self::Result {
        if let Some(tx) = self.tx.take() {
            let _ = tx.send(Err(anyhow!("Event timeout!")));
            ctx.stop();
        }
    }
}

/// Prepare a receiver to return the first event that passes the matcher function from the event
/// bus. You must return the receiver first before triggering any events.
pub fn wait_for_event<E>(
    bus: &Addr<EventBus<E>>,
    matcher: Box<dyn Fn(&E) -> bool + Send + 'static>,
) -> oneshot::Receiver<Result<E>>
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

    let history = HistoryCollector::new().start();

    bus.do_send(Subscribe::new("*", history.clone().recipient()));
    (bus, history)
}
