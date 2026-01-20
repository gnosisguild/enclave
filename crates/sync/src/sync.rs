// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Actor, AsyncContext, Handler, Message};
use e3_events::{prelude::*, trap, BusHandle, EType, EnclaveEventData};
use tracing::info;

#[derive(Clone)]
pub struct BufferedEvent {
    pub block: u64,
    pub event: EnclaveEventData,
}

/// Message from EvmEventReaders containing historical events
#[derive(Message)]
#[rtype(result = "()")]
pub struct HistoricalEvents {
    pub events: Vec<BufferedEvent>,
}

/// Message from EvmEventReaders to register with sync actor
#[derive(Message)]
#[rtype(result = "()")]
pub struct RegisterReader;

/// Message from EvmEventReaders signaling historical sync is complete
#[derive(Message)]
#[rtype(result = "()")]
pub struct ReaderComplete;

/// Union of all messages the Sync actor can receive
#[derive(Message)]
#[rtype(result = "()")]
pub enum SyncMessage {
    RegisterReader,
    ReaderComplete,
    HistoricalEvents(HistoricalEvents),
}

/// Coordinates historical replay across all EvmEventReaders.
/// Replaces HistoricalEventCoordinator with enhanced functionality for future multi-source sync.
pub struct Sync {
    /// Count of readers that have registered
    registered_count: usize,
    /// Count of readers that have completed historical sync
    completed_count: usize,
    /// Buffered events during historical sync
    buffered_events: Vec<BufferedEvent>,
    /// Target to forward events to (typically EventBus)
    target: BusHandle,
    /// Whether we've started forwarding events
    started: bool,
}

impl Sync {
    pub fn new(bus: BusHandle) -> Self {
        Self {
            registered_count: 0,
            completed_count: 0,
            buffered_events: Vec::new(),
            target: bus,
            started: false,
        }
    }
}

impl Actor for Sync {
    type Context = actix::Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // Auto-publish SyncStart when actor starts
        let sync_addr_str = format!("{:?}", ctx.address());
        let sync_start = EnclaveEventData::SyncStart(e3_events::SyncStart {
            sync_address: sync_addr_str,
        });
        trap(EType::Sync, &self.target.clone(), || {
            self.target.publish(sync_start)
        });
    }
}

impl Handler<SyncMessage> for Sync {
    type Result = ();

    fn handle(&mut self, msg: SyncMessage, _ctx: &mut Self::Context) -> Self::Result {
        match msg {
            SyncMessage::RegisterReader => {
                self.registered_count += 1;
                info!("Sync: reader registered (total: {})", self.registered_count);
            }
            SyncMessage::ReaderComplete => {
                self.completed_count += 1;
                info!("Sync: reader completed (total: {})", self.completed_count);

                if self.all_readers_complete() {
                    info!("Sync: all readers complete, publishing ordered events");
                    trap(EType::Sync, &self.target.clone(), || {
                        self.flush_buffered_events()
                    });

                    // Publish SyncEnd to trigger live streaming
                    trap(EType::Sync, &self.target.clone(), || {
                        self.target
                            .publish(EnclaveEventData::SyncEnd(e3_events::SyncEnd))
                    });
                }
            }
            SyncMessage::HistoricalEvents(events) => {
                let event_count = events.events.len();
                self.buffered_events.extend(events.events);
                info!(
                    "Sync: received {} events (total buffered: {})",
                    event_count,
                    self.buffered_events.len()
                );
            }
        }
    }
}

impl Sync {
    fn all_readers_complete(&self) -> bool {
        self.registered_count > 0 && self.registered_count == self.completed_count
    }

    fn flush_buffered_events(&mut self) -> anyhow::Result<()> {
        // Ordering by block number. But we should also consider the tx_index and log_index.
        self.buffered_events.sort_by_key(|e| e.block);

        let count = self.buffered_events.len();
        for BufferedEvent { event, .. } in self.buffered_events.drain(..) {
            self.target.publish(event)?;
        }

        info!("Sync: replay complete, published {} ordered events", count);
        Ok(())
    }
}
