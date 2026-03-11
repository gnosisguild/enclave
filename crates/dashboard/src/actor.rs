// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{
    Actor, Addr, AsyncContext, Context, Handler, Message, MessageResult, Recipient, ResponseFuture,
};
use e3_events::prelude::{Event, EventContextAccessors};
use e3_events::{
    AggregateId, CorrelationId, EnclaveEvent, EnclaveEventData, EventStoreQueryBy,
    EventStoreQueryResponse, EventSubscriber, EventType, SeqAgg,
};
use serde::Serialize;
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};
use tokio::sync::oneshot;

#[derive(Clone, Debug, Serialize)]
pub struct DashboardEvent {
    pub timestamp: String,
    pub event_type: String,
    pub e3_id: Option<String>,
    pub details: String,
    pub is_error: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct NodeMetrics {
    pub e3s_participated: u64,
    pub e3s_completed: u64,
    pub e3s_failed: u64,
    pub current_e3s: Vec<String>,
    pub ticket_balance: Option<String>,
    pub node_address: String,
    pub uptime_seconds: u64,
}

#[derive(Message)]
#[rtype(result = "NodeMetrics")]
pub struct GetMetrics;

/// Query persisted events from the EventStore. Returns all matching events.
#[derive(Message)]
#[rtype(result = "Vec<DashboardEvent>")]
pub struct QueryPersistedEvents {
    pub event_type: Option<String>,
    pub e3_id: Option<String>,
    pub limit: Option<u64>,
}

pub struct DashboardActor {
    // Live metrics from bus subscription
    e3s_participated: u64,
    e3s_completed: u64,
    e3s_failed: u64,
    current_e3s: HashSet<String>,
    ticket_balance: Option<String>,
    node_address: String,
    uptime_started: Instant,
    // Persisted event store query support
    eventstore: Option<Recipient<EventStoreQueryBy<SeqAgg>>>,
    aggregate_ids: Vec<AggregateId>,
    pending_queries: HashMap<CorrelationId, (Instant, oneshot::Sender<Vec<EnclaveEvent>>)>,
    // Fallback ring buffer when no eventstore is available (in-mem / test mode)
    fallback_events: VecDeque<DashboardEvent>,
}

const FALLBACK_MAX_EVENTS: usize = 2000;
const PENDING_QUERY_TIMEOUT: Duration = Duration::from_secs(30);

impl DashboardActor {
    pub fn new(node_address: &str) -> Self {
        Self {
            e3s_participated: 0,
            e3s_completed: 0,
            e3s_failed: 0,
            current_e3s: HashSet::new(),
            ticket_balance: None,
            node_address: node_address.to_string(),
            uptime_started: Instant::now(),
            eventstore: None,
            aggregate_ids: vec![],
            pending_queries: HashMap::new(),
            fallback_events: VecDeque::with_capacity(FALLBACK_MAX_EVENTS),
        }
    }

    pub fn with_eventstore(
        mut self,
        eventstore: Recipient<EventStoreQueryBy<SeqAgg>>,
        aggregate_ids: Vec<AggregateId>,
    ) -> Self {
        self.eventstore = Some(eventstore);
        self.aggregate_ids = aggregate_ids;
        self
    }

    pub fn attach(bus: &impl EventSubscriber<EnclaveEvent>, node_address: &str) -> Addr<Self> {
        let addr = Self::new(node_address).start();
        bus.subscribe(EventType::All, addr.clone().recipient());
        addr
    }

    pub fn attach_with_eventstore(
        bus: &impl EventSubscriber<EnclaveEvent>,
        node_address: &str,
        eventstore: Recipient<EventStoreQueryBy<SeqAgg>>,
        aggregate_ids: Vec<AggregateId>,
    ) -> Addr<Self> {
        let actor = Self::new(node_address).with_eventstore(eventstore, aggregate_ids);
        let addr = actor.start();
        bus.subscribe(EventType::All, addr.clone().recipient());
        addr
    }

    fn push_fallback_event(&mut self, event: DashboardEvent) {
        if self.fallback_events.len() >= FALLBACK_MAX_EVENTS {
            self.fallback_events.pop_front();
        }
        self.fallback_events.push_back(event);
    }

    fn build_metrics(&self) -> NodeMetrics {
        NodeMetrics {
            e3s_participated: self.e3s_participated,
            e3s_completed: self.e3s_completed,
            e3s_failed: self.e3s_failed,
            current_e3s: {
                let mut ids: Vec<String> = self.current_e3s.iter().cloned().collect();
                ids.sort();
                ids
            },
            ticket_balance: self.ticket_balance.clone(),
            node_address: self.node_address.clone(),
            uptime_seconds: self.uptime_started.elapsed().as_secs(),
        }
    }

    fn update_metrics_from_event(&mut self, data: &EnclaveEventData) {
        match data {
            EnclaveEventData::CiphernodeSelected(ref selected) => {
                self.e3s_participated += 1;
                self.current_e3s.insert(selected.e3_id.to_string());
            }
            EnclaveEventData::E3RequestComplete(ref complete) => {
                self.e3s_completed += 1;
                self.current_e3s.remove(&complete.e3_id.to_string());
            }
            EnclaveEventData::E3Failed(ref failed) => {
                self.e3s_failed += 1;
                self.current_e3s.remove(&failed.e3_id.to_string());
            }
            EnclaveEventData::TicketBalanceUpdated(ref updated) => {
                self.ticket_balance = Some(updated.new_balance.to_string());
            }
            _ => {}
        }
    }

    /// Remove pending queries that have been waiting longer than the timeout.
    fn cleanup_stale_queries(&mut self) {
        let now = Instant::now();
        self.pending_queries
            .retain(|_, (created_at, _)| now.duration_since(*created_at) < PENDING_QUERY_TIMEOUT);
    }
}

/// Truncate a string at a safe UTF-8 char boundary.
fn truncate_string(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    // Find the last valid char boundary at or before max_bytes
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &s[..end])
}

fn enclave_event_to_dashboard_event(event: &EnclaveEvent) -> DashboardEvent {
    let data = event.get_data();
    let event_type = data.event_type();
    let e3_id = event.get_e3_id().map(|id| id.to_string());
    let is_error = matches!(data, EnclaveEventData::EnclaveError(_));

    let details = truncate_string(&format!("{:?}", data), 500);

    // Use the event's HLC timestamp converted to a wall-clock estimate.
    // The HLC ts is a u128 packed as [ts_micros(u64) | counter(u32) | node(u32)] in big-endian.
    // The upper 64 bits are microseconds since the UNIX epoch.
    let ts = event.ts();
    let micros = (ts >> 64) as i64;
    let timestamp = chrono::DateTime::from_timestamp_micros(micros)
        .unwrap_or_default()
        .to_rfc3339();

    DashboardEvent {
        timestamp,
        event_type,
        e3_id,
        details,
        is_error,
    }
}

impl Actor for DashboardActor {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for DashboardActor {
    type Result = ();

    fn handle(&mut self, msg: EnclaveEvent, _ctx: &mut Self::Context) -> Self::Result {
        let data = msg.get_data();
        self.update_metrics_from_event(&data);

        // Push to fallback buffer (used when no eventstore is available)
        if self.eventstore.is_none() {
            self.push_fallback_event(enclave_event_to_dashboard_event(&msg));
        }
    }
}

impl Handler<GetMetrics> for DashboardActor {
    type Result = MessageResult<GetMetrics>;

    fn handle(&mut self, _msg: GetMetrics, _ctx: &mut Self::Context) -> Self::Result {
        MessageResult(self.build_metrics())
    }
}

impl Handler<QueryPersistedEvents> for DashboardActor {
    type Result = ResponseFuture<Vec<DashboardEvent>>;

    fn handle(&mut self, msg: QueryPersistedEvents, ctx: &mut Self::Context) -> Self::Result {
        // Clean up any stale pending queries before adding new ones
        self.cleanup_stale_queries();

        // If no eventstore, return from the fallback ring buffer
        let eventstore = match self.eventstore.clone() {
            Some(es) => es,
            None => {
                let events = filter_dashboard_events(
                    self.fallback_events.iter().rev().cloned().collect(),
                    msg.event_type,
                    msg.e3_id,
                    msg.limit,
                );
                return Box::pin(async move { events });
            }
        };

        // Build query across all aggregate IDs, starting from seq 0
        let mut query: HashMap<AggregateId, u64> = HashMap::new();
        for agg_id in &self.aggregate_ids {
            query.insert(*agg_id, 0);
        }
        // Always include the default aggregate (0) for non-chain events
        query.entry(AggregateId::new(0)).or_insert(0);

        let id = CorrelationId::new();
        let (tx, rx) = oneshot::channel();
        self.pending_queries.insert(id, (Instant::now(), tx));

        let limit = msg.limit;
        let event_type_filter = msg.event_type;
        let e3_id_filter = msg.e3_id;

        // Send query to eventstore router, with our own address as receiver.
        // Only pass limit to the eventstore when no filters are active, otherwise
        // the eventstore would truncate results before we can filter them client-side.
        let has_filters = event_type_filter.as_ref().is_some_and(|s| !s.is_empty())
            || e3_id_filter.as_ref().is_some_and(|s| !s.is_empty());
        let mut query_msg = EventStoreQueryBy::<SeqAgg>::new(id, query, ctx.address().recipient());
        if !has_filters {
            if let Some(l) = limit {
                query_msg = query_msg.with_limit(l);
            }
        }

        if let Err(e) = eventstore.try_send(query_msg) {
            tracing::error!("Failed to send query to eventstore: {}", e);
            // Remove pending query and return empty
            self.pending_queries.remove(&id);
            return Box::pin(async move { vec![] });
        }

        Box::pin(async move {
            let result = tokio::time::timeout(PENDING_QUERY_TIMEOUT, rx).await;
            match result {
                Ok(Ok(events)) => {
                    let dashboard_events: Vec<DashboardEvent> = events
                        .iter()
                        .rev()
                        .map(enclave_event_to_dashboard_event)
                        .collect();
                    filter_dashboard_events(
                        dashboard_events,
                        event_type_filter,
                        e3_id_filter,
                        limit,
                    )
                }
                _ => vec![],
            }
        })
    }
}

impl Handler<EventStoreQueryResponse> for DashboardActor {
    type Result = ();

    fn handle(&mut self, msg: EventStoreQueryResponse, _ctx: &mut Self::Context) -> Self::Result {
        if let Some((_, tx)) = self.pending_queries.remove(&msg.id()) {
            let _ = tx.send(msg.into_events());
        }
    }
}

fn filter_dashboard_events(
    events: Vec<DashboardEvent>,
    event_type: Option<String>,
    e3_id: Option<String>,
    limit: Option<u64>,
) -> Vec<DashboardEvent> {
    let limit = limit.unwrap_or(500) as usize;
    events
        .into_iter()
        .filter(|event| {
            if let Some(ref et) = event_type {
                if !et.is_empty() && !event.event_type.to_lowercase().contains(&et.to_lowercase()) {
                    return false;
                }
            }
            if let Some(ref id) = e3_id {
                if !id.is_empty() {
                    match &event.e3_id {
                        Some(eid) => {
                            if !eid.contains(id.as_str()) {
                                return false;
                            }
                        }
                        None => return false,
                    }
                }
            }
            true
        })
        .take(limit)
        .collect()
}
