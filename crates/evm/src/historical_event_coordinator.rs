// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::Addr;
use e3_events::{EnclaveEvent, EventBus};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use tracing::info;

#[derive(Clone)]
struct BufferedEvent {
    block: u64,
    event: EnclaveEvent,
}

/// Coordinates historical replay across all EvmEventReaders.
/// Buffers historical events, then sorts + publishes once all readers finish.
pub struct HistoricalEventCoordinator {
    pending_readers: AtomicUsize,
    events: Mutex<Vec<BufferedEvent>>,
    bus: Addr<EventBus<EnclaveEvent>>,
}

impl HistoricalEventCoordinator {
    pub fn new(reader_count: usize, bus: Addr<EventBus<EnclaveEvent>>) -> Arc<Self> {
        Arc::new(Self {
            pending_readers: AtomicUsize::new(reader_count),
            events: Mutex::new(Vec::new()),
            bus,
        })
    }

    /// Called by readers while still in "historical" phase.
    pub fn buffer_event(&self, block: Option<u64>, event: &EnclaveEvent) {
        let Some(block) = block else {
            // If block is missing, treat as 0 or skip – here we skip to avoid weird ordering.
            return;
        };

        let mut guard = self
            .events
            .lock()
            .expect("HistoricalEventCoordinator.events poisoned");

        guard.push(BufferedEvent {
            block,
            event: event.clone(),
        });
    }

    /// Called once per reader when it has finished fetching historical logs.
    /// When the last reader calls this, we sort + publish everything.
    pub fn reader_finished(&self) {
        let remaining = self.pending_readers.fetch_sub(1, Ordering::SeqCst);

        // `remaining` is the *old* value. When it hits 1 -> this call makes it 0.
        if remaining != 1 {
            return;
        }

        let mut events = self
            .events
            .lock()
            .expect("HistoricalEventCoordinator.events poisoned");

        // Ordering by block number. But we should also consider the tx_index and log_index.
        events.sort_by_key(|e| e.block);

        let count = events.len();
        for BufferedEvent { event, .. } in events.drain(..) {
            self.bus.do_send(event);
        }

        info!(
            "HistoricalEventCoordinator: replay complete, published {} ordered events",
            count
        );
    }
}
