// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{EnclaveEvmEvent, EvmEventProcessor, HistoricalSyncComplete};
use actix::{Actor, Addr, Handler};
use bloom::{BloomFilter, ASMS};
use e3_events::CorrelationId;
use tracing::info;

pub struct FixHistoricalOrder {
    dest: EvmEventProcessor,
    pending_sync_complete: Option<EnclaveEvmEvent>,
    seen_ids: BloomFilter,
}

impl FixHistoricalOrder {
    pub fn new(dest: impl Into<EvmEventProcessor>) -> Self {
        Self {
            dest: dest.into(),
            pending_sync_complete: None,
            seen_ids: BloomFilter::with_rate(0.001, 10_000),
        }
    }

    pub fn setup(dest: impl Into<EvmEventProcessor>) -> Addr<Self> {
        Self::new(dest).start()
    }

    fn send_pending(&mut self) {
        if let Some(EnclaveEvmEvent::HistoricalSyncComplete(HistoricalSyncComplete {
            prev_event: Some(ref id),
            ..
        })) = self.pending_sync_complete
        {
            if self.seen_ids.contains(id) {
                info!("Forwarding historical send complete event");
                self.dest
                    .do_send(self.pending_sync_complete.take().unwrap());
            }
        }
    }

    fn track_id(&mut self, id: CorrelationId) {
        self.seen_ids.insert(&id);
    }
}

impl Actor for FixHistoricalOrder {
    type Context = actix::Context<Self>;
}

impl Handler<EnclaveEvmEvent> for FixHistoricalOrder {
    type Result = ();

    fn handle(&mut self, msg: EnclaveEvmEvent, _ctx: &mut Self::Context) {
        let id = msg.get_id();
        info!("Receiving EnclaveEvmEvent event({})", msg.get_id());
        match msg {
            none_hist @ EnclaveEvmEvent::HistoricalSyncComplete(HistoricalSyncComplete {
                prev_event: None,
                ..
            }) => {
                info!(
                    "Historical order event({}) has no previous event. Forwarding...",
                    id
                );
                self.dest.do_send(none_hist);
            }
            hist @ EnclaveEvmEvent::HistoricalSyncComplete(HistoricalSyncComplete {
                prev_event: Some(prev),
                ..
            }) => {
                info!(
                    "Historical order event({}) has previous event({}). Buffering...",
                    id, prev
                );

                self.pending_sync_complete = Some(hist);
            }
            EnclaveEvmEvent::Processed(id) => self.track_id(id),
            other => {
                info!("Forwarding event({})", other.get_id());
                self.track_id(other.get_id());
                self.dest.do_send(other);
            }
        }
        self.send_pending();
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::EvmLog;

    use super::*;
    use actix::prelude::*;
    use alloy_primitives::Address;
    use tokio::{sync::mpsc, time::sleep};

    struct Collector(mpsc::UnboundedSender<EnclaveEvmEvent>);

    impl Actor for Collector {
        type Context = Context<Self>;
    }

    impl Handler<EnclaveEvmEvent> for Collector {
        type Result = ();
        fn handle(&mut self, msg: EnclaveEvmEvent, _ctx: &mut Self::Context) {
            let _ = self.0.send(msg);
        }
    }

    #[actix::test]
    async fn test_reorders_sync_complete_after_referenced_event() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let fix = FixHistoricalOrder::setup(Collector(tx).start());

        let log_1 = EnclaveEvmEvent::Log(EvmLog::test_log(Address::ZERO, 1, 1));
        let log_2 = EnclaveEvmEvent::Log(EvmLog::test_log(Address::ZERO, 2, 2));
        let log_3 = EnclaveEvmEvent::Log(EvmLog::test_log(Address::ZERO, 3, 3));

        let sync_complete = EnclaveEvmEvent::HistoricalSyncComplete(HistoricalSyncComplete::new(
            1,
            Some(log_3.get_id()),
        ));

        // Send logs 1, 2, 3
        fix.send(log_1.clone()).await.unwrap();
        // Send sync complete FIRST (out of order - references log_3 which hasn't been seen)
        fix.send(sync_complete.clone()).await.unwrap();
        fix.send(log_2.clone()).await.unwrap();
        fix.send(log_3.clone()).await.unwrap();

        sleep(Duration::from_secs(1)).await;

        // Collect results
        let mut received = vec![];
        while let Ok(msg) = rx.try_recv() {
            received.push(msg);
        }

        // The sync complete should have been held until log_3 was seen
        assert_eq!(received.len(), 4);
        assert_eq!(received[0], log_1);
        assert_eq!(received[1], log_2);
        assert_eq!(received[2], log_3);
        assert_eq!(received[3], sync_complete);
    }
}
