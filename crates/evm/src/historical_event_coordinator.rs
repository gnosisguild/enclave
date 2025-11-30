// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::EnclaveEvmEvent;
use actix::prelude::*;
use e3_events::{prelude::*, EnclaveEvent, EnclaveEventData, BusHandle};
use tracing::info;

#[derive(Clone)]
struct BufferedEvent {
    block: u64,
    event: EnclaveEventData,
}

/// Message to start forwarding buffered events after all readers have registered
#[derive(Message)]
#[rtype(result = "()")]
pub struct CoordinatorStart;

/// Coordinates historical replay across all EvmEventReaders.
/// Buffers historical events, then sorts + publishes once all readers finish.
pub struct HistoricalEventCoordinator {
    /// Count of readers that have registered
    registered_count: usize,
    /// Count of readers that have completed historical sync
    completed_count: usize,
    /// Buffered events during historical sync
    buffered_events: Vec<BufferedEvent>,
    /// Target to forward events to (typically EventBus)
    target: BusHandle<EnclaveEvent>,
    /// Whether we've started forwarding (after Start message)
    started: bool,
}

impl HistoricalEventCoordinator {
    pub fn new(target: BusHandle<EnclaveEvent>) -> Self {
        Self {
            registered_count: 0,
            completed_count: 0,
            buffered_events: Vec::new(),
            target,
            started: false,
        }
    }

    pub fn setup(target: BusHandle<EnclaveEvent>) -> Addr<Self> {
        Self::new(target).start()
    }

    fn all_readers_complete(&self) -> bool {
        self.registered_count > 0 && self.registered_count == self.completed_count
    }

    fn flush_buffered_events(&mut self) {
        // Ordering by block number. But we should also consider the tx_index and log_index.
        self.buffered_events.sort_by_key(|e| e.block);

        let count = self.buffered_events.len();
        for BufferedEvent { event, .. } in self.buffered_events.drain(..) {
            self.target.dispatch(event);
        }

        info!(
            "HistoricalEventCoordinator: replay complete, published {} ordered events",
            count
        );
    }
}

impl Actor for HistoricalEventCoordinator {
    type Context = Context<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        info!("HistoricalEventCoordinator started");
    }
}

impl Handler<EnclaveEvmEvent> for HistoricalEventCoordinator {
    type Result = ();

    fn handle(&mut self, msg: EnclaveEvmEvent, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            EnclaveEvmEvent::RegisterReader => {
                self.registered_count += 1;
                info!(
                    total_registered = self.registered_count,
                    "Reader registered with coordinator"
                );
            }

            EnclaveEvmEvent::HistoricalSyncComplete => {
                self.completed_count += 1;
                info!(
                    completed = self.completed_count,
                    total_registered = self.registered_count,
                    "Reader completed historical sync"
                );

                if self.started && self.all_readers_complete() {
                    info!("All readers completed historical sync, flushing buffered events");
                    self.flush_buffered_events();
                }
            }

            EnclaveEvmEvent::Event { event, block } => {
                if !self.started || !self.all_readers_complete() {
                    if let Some(block) = block {
                        self.buffered_events.push(BufferedEvent { block, event });
                    }
                } else {
                    self.target.dispatch(event);
                }
            }
        }
    }
}

impl Handler<CoordinatorStart> for HistoricalEventCoordinator {
    type Result = ();

    fn handle(&mut self, _msg: CoordinatorStart, _ctx: &mut Self::Context) -> Self::Result {
        info!(
            registered_readers = self.registered_count,
            "Starting HistoricalEventCoordinator"
        );
        self.started = true;

        if self.all_readers_complete() {
            self.flush_buffered_events();
        }
    }
}
