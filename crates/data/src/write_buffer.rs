// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Actor, Handler, Message, Recipient};
use e3_events::{AggregateId, CommitSnapshot, EventContextAccessors};
use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{Insert, InsertBatch};

struct AggregateBuffer {
    buffer: Vec<Insert>,
}

impl AggregateBuffer {
    fn new() -> Self {
        Self { buffer: Vec::new() }
    }
}

pub struct WriteBuffer {
    /// Destination recipient for batched inserts
    dest: Option<Recipient<InsertBatch>>,
    /// Per-aggregate buffers for organizing inserts
    aggregate_buffers: HashMap<AggregateId, AggregateBuffer>,
    /// Per-aggregate wait time (microseconds) before sending inserts to destination
    config: HashMap<AggregateId, u64>,
}

impl Actor for WriteBuffer {
    type Context = actix::Context<Self>;
}

impl WriteBuffer {
    pub fn new() -> Self {
        Self {
            dest: None,
            aggregate_buffers: HashMap::new(),
            config: HashMap::new(),
        }
    }

    pub fn with_config(config: HashMap<AggregateId, u64>) -> Self {
        Self {
            dest: None,
            aggregate_buffers: HashMap::new(),
            config,
        }
    }
}

impl Handler<ForwardTo> for WriteBuffer {
    type Result = ();
    fn handle(&mut self, msg: ForwardTo, _: &mut Self::Context) -> Self::Result {
        self.dest = Some(msg.dest())
    }
}

impl Handler<Insert> for WriteBuffer {
    type Result = ();

    fn handle(&mut self, msg: Insert, _: &mut Self::Context) -> Self::Result {
        let aggregate_id = if let Some(event_ctx) = msg.ctx() {
            event_ctx.aggregate_id().clone()
        } else {
            AggregateId::new(0)
        };

        let agg_buffer = self
            .aggregate_buffers
            .entry(aggregate_id)
            .or_insert_with(|| AggregateBuffer::new());
        agg_buffer.buffer.push(msg.clone());
    }
}

impl Handler<CommitSnapshot> for WriteBuffer {
    type Result = ();

    fn handle(&mut self, _msg: CommitSnapshot, _: &mut Self::Context) -> Self::Result {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_secs(0))
            .as_millis();

        if let Some(ref dest) = self.dest {
            let mut aggregates_to_remove = Vec::new();

            for (aggregate_id, agg_buffer) in &mut self.aggregate_buffers {
                let delay_micros = self.config.get(aggregate_id).copied().unwrap_or(0);
                let delay_ms = delay_micros / 1000;
                let cutoff_time = now.saturating_sub(delay_ms as u128);

                let mut expired_inserts = Vec::new();
                let mut remaining_inserts = Vec::new();

                for insert in &agg_buffer.buffer {
                    if let Some(ctx) = insert.ctx() {
                        if ctx.ts() < cutoff_time {
                            expired_inserts.push(insert.clone());
                        } else {
                            remaining_inserts.push(insert.clone());
                        }
                    } else {
                        remaining_inserts.push(insert.clone());
                    }
                }

                if !expired_inserts.is_empty() {
                    let batch = InsertBatch::new(expired_inserts);
                    dest.do_send(batch);
                }

                agg_buffer.buffer = remaining_inserts;

                if agg_buffer.buffer.is_empty() {
                    aggregates_to_remove.push(aggregate_id.clone());
                }
            }

            for aggregate_id in aggregates_to_remove {
                self.aggregate_buffers.remove(&aggregate_id);
            }
        }
    }
}

#[derive(Message)]
#[rtype("()")]
pub struct ForwardTo(Recipient<InsertBatch>);

impl ForwardTo {
    pub fn new(dest: impl Into<Recipient<InsertBatch>>) -> Self {
        Self(dest.into())
    }

    pub fn dest(self) -> Recipient<InsertBatch> {
        self.0
    }
}
