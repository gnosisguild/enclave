// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
use super::{
    batch::{Batch, Flush},
    timelock_queue::{Clock, StartTimelock},
    AggregateConfig, UpdateDestination,
};
use crate::{
    trap, AggregateId, EType, EnclaveEvent, EventContextAccessors, EventContextSeq, Insert,
    InsertBatch, PanicDispatcher, Sequenced, StoreKeys,
};
use actix::{Actor, Addr, Handler, Message, Recipient};
use anyhow::Context;
use e3_utils::MAILBOX_LIMIT;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tracing::{debug, info, trace, warn};

type Seq = u64;

#[derive(Message)]
#[rtype(result = "()")]
pub struct FlushSeq(pub Seq);

impl FlushSeq {
    pub fn seq(&self) -> u64 {
        self.0
    }
}

pub struct BatchRouter {
    config: AggregateConfig,
    aggregates: HashMap<Seq, AggregateId>,
    batches: HashMap<Seq, Addr<Batch>>,
    block_height_seen: HashMap<AggregateId, u64>,
    timelock_queue: Recipient<StartTimelock>,
    db: Recipient<InsertBatch>,
    clock: Arc<dyn Clock>,
}

impl Actor for BatchRouter {
    type Context = actix::Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.set_mailbox_capacity(MAILBOX_LIMIT);
    }
}

impl BatchRouter {
    pub fn new(
        config: &AggregateConfig,
        timelock_queue: impl Into<Recipient<StartTimelock>>,
        db: impl Into<Recipient<InsertBatch>>,
    ) -> Self {
        Self::with_clock(
            config,
            timelock_queue,
            db,
            Arc::new(super::timelock_queue::SystemClock),
        )
    }

    pub fn with_clock(
        config: &AggregateConfig,
        timelock_queue: impl Into<Recipient<StartTimelock>>,
        db: impl Into<Recipient<InsertBatch>>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            batches: HashMap::new(),
            aggregates: HashMap::new(),
            config: config.clone(),
            timelock_queue: timelock_queue.into(),
            block_height_seen: HashMap::new(),
            db: db.into(),
            clock,
        }
    }

    fn get_highest_block(&mut self, agg: AggregateId, block: Option<u64>) -> u64 {
        let highest = block
            .into_iter()
            .chain(self.block_height_seen.get(&agg).copied())
            .max()
            .unwrap_or(0);

        self.block_height_seen.insert(agg, highest);
        highest
    }
}

impl Handler<Insert> for BatchRouter {
    type Result = ();
    fn handle(&mut self, msg: Insert, _: &mut Self::Context) -> Self::Result {
        trap(EType::IO, &PanicDispatcher::new(), || {
            // Messages without context go straight to disk
            // This is probably direct datastore manipulation
            let Some(ctx) = msg.ctx() else {
                debug!("Message without context. Flushing straight to disk.");
                self.db.try_send(InsertBatch::new(vec![msg]))?;
                return Ok(());
            };

            // Route to existing batch, or fall back to disk
            match self.batches.get(&ctx.seq()) {
                Some(batch) => {
                    debug!("Forwarding to batch actor for seq={}", ctx.seq());
                    batch.try_send(msg)?;
                }
                // This must mean that this insert is late
                None => {
                    debug!(
                        "No batch available for seq={} assuming this is late. Flushing to disk.",
                        ctx.seq()
                    );
                    self.db.try_send(InsertBatch::new(vec![msg]))?;
                }
            }
            Ok(())
        })
    }
}

impl Handler<EnclaveEvent<Sequenced>> for BatchRouter {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent<Sequenced>, _: &mut Self::Context) -> Self::Result {
        let ec = msg.get_ctx();
        trap(EType::IO, &PanicDispatcher::new(), || {
            let prev_seq = ec.seq() - 1;
            if self.batches.contains_key(&prev_seq) {
                let prev_agg = self
                    .aggregates
                    .get(&prev_seq)
                    .context("invariant: prev_agg MUST exist if batches has a batch")?;

                debug!(
                    "Preparing timelock to clear batch for seq={}, agg={}",
                    prev_seq, prev_agg
                );
                let delay = self.config.get_delay(prev_agg);

                let now = Duration::from_micros(self.clock.now_micros());

                self.timelock_queue
                    .try_send(StartTimelock::new(prev_seq, now, delay))?;
            }

            debug!("Creating batch for {}", ec.seq());
            let agg_id = ec.aggregate_id();
            let highest_block = self.get_highest_block(agg_id, ec.block());
            let batch = Batch::spawn(
                self.db.clone(),
                vec![
                    Insert::new_with_context(
                        &StoreKeys::aggregate_seq(agg_id),
                        encode_u64(ec.seq()),
                        ec.clone(),
                    ),
                    Insert::new_with_context(
                        &StoreKeys::aggregate_block(agg_id),
                        encode_u64(highest_block),
                        ec.clone(),
                    ),
                    Insert::new_with_context(
                        &StoreKeys::aggregate_ts(agg_id),
                        encode_u128(ec.ts()),
                        ec.clone(),
                    ),
                ],
            );

            self.batches.insert(ec.seq(), batch);
            self.aggregates.insert(ec.seq(), ec.aggregate_id());

            Ok(())
        })
    }
}

impl Handler<FlushSeq> for BatchRouter {
    type Result = ();
    fn handle(&mut self, msg: FlushSeq, _: &mut Self::Context) -> Self::Result {
        trap(EType::IO, &PanicDispatcher::new(), || {
            info!("Flushing sequence... {}", msg.seq());
            if let Some(batch) = self.batches.get(&msg.seq()) {
                batch.try_send(Flush)?;
                self.batches.remove(&msg.seq());
                self.aggregates.remove(&msg.seq());
            }
            Ok(())
        })
    }
}

impl Handler<UpdateDestination> for BatchRouter {
    type Result = ();
    fn handle(&mut self, msg: UpdateDestination, _: &mut Self::Context) -> Self::Result {
        trap(EType::IO, &PanicDispatcher::new(), || {
            self.db = msg.0;
            Ok(())
        })
    }
}

/// Encode the same as bincode without using a result
fn encode_u64(value: u64) -> Vec<u8> {
    value.to_le_bytes().to_vec()
}

/// Encode the same as bincode without using a result
fn encode_u128(value: u128) -> Vec<u8> {
    value.to_le_bytes().to_vec()
}
