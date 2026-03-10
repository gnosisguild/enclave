// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{
    events::{StoreEventRequested, StoreEventResponse},
    EnclaveEvent, EventContextAccessors, EventLog, EventStoreFilter, EventStoreQueryBy,
    EventStoreQueryResponse, Seq, SequenceIndex, Sequenced, Ts, Unsequenced,
};
use actix::{Actor, Handler};
use anyhow::{bail, Result};
use tracing::{error, warn};

const MAX_STORAGE_ERRORS: u64 = 10;

pub struct EventStore<I: SequenceIndex, L: EventLog> {
    index: I,
    log: L,
    storage_errors: u64,
}

impl<I: SequenceIndex, L: EventLog> EventStore<I, L> {
    /// Attempt to store an event. Returns the sequenced event on success,
    /// `None` if the event was a duplicate, or an error on failure.
    pub fn store_event(
        &mut self,
        event: EnclaveEvent<Unsequenced>,
    ) -> Result<Option<EnclaveEvent<Sequenced>>> {
        let ts = event.ts();
        if let Some(_) = self.index.get(ts)? {
            warn!("Event already stored at timestamp {ts}! This might happen when recovering from a snapshot. Skipping storage");
            self.storage_errors += 1;
            if self.storage_errors > MAX_STORAGE_ERRORS {
                bail!(
                    "The eventstore had too many storage errors! {}",
                    self.storage_errors
                );
            }
            return Ok(None);
        }
        let seq = self.log.append(&event)?;
        self.index.insert(ts, seq)?;
        Ok(Some(event.into_sequenced(seq)))
    }

    fn collect_events(
        &self,
        iter: Box<dyn Iterator<Item = (u64, EnclaveEvent<Unsequenced>)>>,
        filter: Option<EventStoreFilter>,
        limit: Option<u64>,
    ) -> Vec<EnclaveEvent<Sequenced>> {
        let iter = iter.map(|(s, e)| e.into_sequenced(s));

        match filter {
            Some(EventStoreFilter::Source(source)) => {
                let iter = iter.filter(move |e| e.get_ctx().source() == source);
                match limit {
                    Some(lim) => iter.take(lim as usize).collect(),
                    None => iter.collect(),
                }
            }
            None => match limit {
                Some(lim) => iter.take(lim as usize).collect(),
                None => iter.collect(),
            },
        }
    }

    /// Query events by timestamp. Returns events at or after the given timestamp.
    pub fn query_by_ts(
        &self,
        query: u128,
        filter: Option<EventStoreFilter>,
        limit: Option<u64>,
    ) -> Result<Vec<EnclaveEvent<Sequenced>>> {
        let Some(seq) = self.index.seek(query)? else {
            return Ok(vec![]);
        };
        Ok(self.collect_events(self.log.read_from(seq), filter, limit))
    }

    /// Query events by sequence number. Returns events at or after the given sequence.
    pub fn query_by_seq(
        &self,
        query: u64,
        filter: Option<EventStoreFilter>,
        limit: Option<u64>,
    ) -> Vec<EnclaveEvent<Sequenced>> {
        self.collect_events(self.log.read_from(query), filter, limit)
    }
}

impl<I: SequenceIndex, L: EventLog> EventStore<I, L> {
    pub fn new(index: I, log: L) -> Self {
        Self {
            index,
            log,
            storage_errors: 0,
        }
    }
}

impl<I: SequenceIndex, L: EventLog> Actor for EventStore<I, L> {
    type Context = actix::Context<Self>;
}

impl<I: SequenceIndex, L: EventLog> Handler<StoreEventRequested> for EventStore<I, L> {
    type Result = ();
    fn handle(&mut self, msg: StoreEventRequested, _: &mut Self::Context) -> Self::Result {
        match self.store_event(msg.event) {
            Ok(Some(sequenced)) => {
                msg.sender.do_send(StoreEventResponse(sequenced));
            }
            Ok(None) => {} // duplicate — already warned inside store_event
            Err(e) => {
                error!("Event storage failed: {e}");
                panic!("Unrecoverable event storage failure: {e}");
            }
        }
    }
}

impl<I: SequenceIndex, L: EventLog> Handler<EventStoreQueryBy<Ts>> for EventStore<I, L> {
    type Result = ();
    fn handle(&mut self, msg: EventStoreQueryBy<Ts>, _: &mut Self::Context) -> Self::Result {
        let query = msg.query();
        let id = msg.id();
        let limit = msg.limit();
        let filter = msg.filter().cloned();
        let sender = msg.sender();
        match self.query_by_ts(query, filter, limit) {
            Ok(evts) => {
                if let Err(e) = sender.try_send(EventStoreQueryResponse::new(id, evts)) {
                    error!("{e}");
                }
            }
            Err(e) => error!("{e}"),
        }
    }
}

impl<I: SequenceIndex, L: EventLog> Handler<EventStoreQueryBy<Seq>> for EventStore<I, L> {
    type Result = ();
    fn handle(&mut self, msg: EventStoreQueryBy<Seq>, _: &mut Self::Context) -> Self::Result {
        let id = msg.id();
        let query = msg.query();
        let limit = msg.limit();
        let filter = msg.filter().cloned();
        let sender = msg.sender();
        let evts = self.query_by_seq(query, filter, limit);
        if let Err(e) = sender.try_send(EventStoreQueryResponse::new(id, evts)) {
            error!("{e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{EventConstructorWithTimestamp, EventSource, TestEvent};

    use super::*;
    use anyhow::Result;
    use std::collections::BTreeMap;

    // ---------------------------------------------------------------------------
    // Mock SequenceIndex backed by BTreeMap
    // ---------------------------------------------------------------------------
    struct MockIndex(BTreeMap<u128, u64>);

    impl MockIndex {
        fn new() -> Self {
            Self(BTreeMap::new())
        }
    }

    impl SequenceIndex for MockIndex {
        fn insert(&mut self, key: u128, value: u64) -> Result<()> {
            self.0.insert(key, value);
            Ok(())
        }

        fn get(&self, key: u128) -> Result<Option<u64>> {
            Ok(self.0.get(&key).copied())
        }

        fn seek(&self, key: u128) -> Result<Option<u64>> {
            Ok(self.0.range(key..).next().map(|(_, &v)| v))
        }
    }

    // ---------------------------------------------------------------------------
    // Mock EventLog backed by Vec
    // ---------------------------------------------------------------------------
    struct MockLog(Vec<EnclaveEvent<Unsequenced>>);

    impl MockLog {
        fn new() -> Self {
            Self(Vec::new())
        }
    }

    impl EventLog for MockLog {
        fn append(&mut self, event: &EnclaveEvent<Unsequenced>) -> Result<u64> {
            let seq = self.0.len() as u64;
            self.0.push(event.clone());
            Ok(seq)
        }

        fn read_from(
            &self,
            from: u64,
        ) -> Box<dyn Iterator<Item = (u64, EnclaveEvent<Unsequenced>)>> {
            let items: Vec<_> = self
                .0
                .iter()
                .enumerate()
                .filter(move |(i, _)| *i as u64 >= from)
                .map(|(i, e)| (i as u64, e.clone()))
                .collect();
            Box::new(items.into_iter())
        }
    }

    // ---------------------------------------------------------------------------
    // Test helpers
    // ---------------------------------------------------------------------------
    fn make_event(ts: u128, source: EventSource) -> EnclaveEvent<Unsequenced> {
        EnclaveEvent::<Unsequenced>::new_with_timestamp(
            TestEvent::new("test", 1).into(),
            None,
            ts,
            None,
            source,
        )
    }

    fn make_local_event(ts: u128) -> EnclaveEvent<Unsequenced> {
        make_event(ts, EventSource::Local)
    }

    fn make_network_event(ts: u128) -> EnclaveEvent<Unsequenced> {
        make_event(ts, EventSource::Net)
    }

    fn new_store() -> EventStore<MockIndex, MockLog> {
        EventStore::new(MockIndex::new(), MockLog::new())
    }

    fn populated_store(events: &[EnclaveEvent<Unsequenced>]) -> EventStore<MockIndex, MockLog> {
        let mut store = new_store();
        for event in events {
            store.store_event(event.clone()).unwrap();
        }
        store
    }

    // ===========================================================================
    // store_event
    // ===========================================================================

    #[test]
    fn store_event_returns_sequenced_event() {
        let mut store = new_store();
        let event = make_local_event(100);

        let result = store.store_event(event).unwrap().unwrap();

        assert_eq!(result.get_ctx().ts(), 100);
    }

    #[test]
    fn store_event_assigns_incrementing_sequence_numbers() {
        let mut store = new_store();

        let _a = store.store_event(make_local_event(100)).unwrap().unwrap();
        let _b = store.store_event(make_local_event(200)).unwrap().unwrap();
        let _c = store.store_event(make_local_event(300)).unwrap().unwrap();

        assert_eq!(store.index.get(100).unwrap(), Some(0));
        assert_eq!(store.index.get(200).unwrap(), Some(1));
        assert_eq!(store.index.get(300).unwrap(), Some(2));
    }

    #[test]
    fn store_event_appends_to_log() {
        let mut store = new_store();
        store.store_event(make_local_event(100)).unwrap();
        store.store_event(make_local_event(200)).unwrap();

        let logged: Vec<_> = store.log.read_from(0).collect();
        assert_eq!(logged.len(), 2);
    }

    #[test]
    fn store_event_returns_none_for_duplicate_timestamp() {
        let mut store = new_store();
        store.store_event(make_local_event(100)).unwrap();

        let result = store.store_event(make_local_event(100)).unwrap();

        assert!(result.is_none());
        assert_eq!(store.storage_errors, 1);
        // Log should still have only one event
        assert_eq!(store.log.read_from(0).count(), 1);
    }

    #[test]
    fn store_event_bails_after_max_storage_errors() {
        let mut store = new_store();
        store.store_event(make_local_event(100)).unwrap();

        for _ in 0..MAX_STORAGE_ERRORS {
            let result = store.store_event(make_local_event(100)).unwrap();
            assert!(result.is_none());
        }

        assert_eq!(store.storage_errors, MAX_STORAGE_ERRORS);

        let result = store.store_event(make_local_event(100));
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("too many storage errors"));
    }

    // ===========================================================================
    // query_by_seq
    // ===========================================================================

    #[test]
    fn seq_query_returns_all_events() {
        let store = populated_store(&[
            make_local_event(100),
            make_local_event(200),
            make_local_event(300),
        ]);

        let events = store.query_by_seq(0, None, None);

        assert_eq!(events.len(), 3);
    }

    #[test]
    fn seq_query_reads_from_given_offset() {
        let store = populated_store(&[
            make_local_event(100),
            make_local_event(200),
            make_local_event(300),
            make_local_event(400),
        ]);

        let events = store.query_by_seq(2, None, None);

        assert_eq!(events.len(), 2);
    }

    #[test]
    fn seq_query_with_source_filter() {
        let store = populated_store(&[
            make_local_event(100),
            make_network_event(200),
            make_local_event(300),
            make_network_event(400),
        ]);

        let events =
            store.query_by_seq(0, Some(EventStoreFilter::Source(EventSource::Local)), None);

        assert_eq!(events.len(), 2);
        for e in &events {
            assert_eq!(e.get_ctx().source(), EventSource::Local);
        }
    }

    #[test]
    fn seq_query_with_limit() {
        let store = populated_store(&[
            make_local_event(100),
            make_local_event(200),
            make_local_event(300),
            make_local_event(400),
            make_local_event(500),
        ]);

        let events = store.query_by_seq(0, None, Some(2));

        assert_eq!(events.len(), 2);
    }

    #[test]
    fn seq_query_with_filter_and_limit() {
        let store = populated_store(&[
            make_local_event(100),
            make_network_event(200),
            make_local_event(300),
            make_local_event(400),
            make_network_event(500),
        ]);

        let events = store.query_by_seq(
            0,
            Some(EventStoreFilter::Source(EventSource::Local)),
            Some(2),
        );

        assert_eq!(events.len(), 2);
        for e in &events {
            assert_eq!(e.get_ctx().source(), EventSource::Local);
        }
    }

    #[test]
    fn seq_query_on_empty_log_returns_empty() {
        let store = new_store();

        let events = store.query_by_seq(0, None, None);

        assert!(events.is_empty());
    }

    // ===========================================================================
    // query_by_ts
    // ===========================================================================

    #[test]
    fn ts_query_returns_events_from_exact_timestamp() {
        let store = populated_store(&[
            make_local_event(100),
            make_local_event(200),
            make_local_event(300),
            make_local_event(400),
        ]);

        let events = store.query_by_ts(200, None, None).unwrap();

        assert_eq!(events.len(), 3);
    }

    #[test]
    fn ts_query_seeks_to_nearest_future_timestamp() {
        let store = populated_store(&[
            make_local_event(100),
            make_local_event(300),
            make_local_event(500),
        ]);

        // ts=200 has no match; seek finds ts=300 onwards
        let events = store.query_by_ts(200, None, None).unwrap();

        assert_eq!(events.len(), 2);
    }

    #[test]
    fn ts_query_returns_empty_when_no_matching_timestamp() {
        let store = new_store();

        let events = store.query_by_ts(999, None, None).unwrap();

        assert!(events.is_empty());
    }

    #[test]
    fn ts_query_returns_empty_when_past_all_events() {
        let store = populated_store(&[make_local_event(100), make_local_event(200)]);

        let events = store.query_by_ts(999, None, None).unwrap();

        assert!(events.is_empty());
    }

    #[test]
    fn ts_query_with_filter() {
        let store = populated_store(&[
            make_local_event(100),
            make_network_event(200),
            make_local_event(300),
        ]);

        let events = store
            .query_by_ts(100, Some(EventStoreFilter::Source(EventSource::Net)), None)
            .unwrap();

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].get_ctx().source(), EventSource::Net);
    }

    #[test]
    fn ts_query_with_limit() {
        let store = populated_store(&[
            make_local_event(100),
            make_local_event(200),
            make_local_event(300),
            make_local_event(400),
        ]);

        let events = store.query_by_ts(100, None, Some(2)).unwrap();

        assert_eq!(events.len(), 2);
    }
}
