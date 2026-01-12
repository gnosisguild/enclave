// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Actor, Handler, Message, Recipient};
use e3_events::{AggregateId, CommitSnapshot, EventContextAccessors};
use std::collections::HashMap;

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
    /// Buffer for storing individual inserts
    buffer: Vec<Insert>,
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
            buffer: Vec::new(),
            aggregate_buffers: HashMap::new(),
            config: HashMap::new(),
        }
    }

    pub fn with_config(config: HashMap<AggregateId, u64>) -> Self {
        Self {
            dest: None,
            buffer: Vec::new(),
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
        if let Some(event_ctx) = msg.ctx() {
            let aggregate_id = event_ctx.aggregate_id().clone();
            let agg_buffer = self
                .aggregate_buffers
                .entry(aggregate_id)
                .or_insert_with(|| AggregateBuffer::new());
            agg_buffer.buffer.push(msg.clone());
        }
        self.buffer.push(msg);
    }
}

impl Handler<CommitSnapshot> for WriteBuffer {
    type Result = ();

    fn handle(&mut self, msg: CommitSnapshot, _: &mut Self::Context) -> Self::Result {
        if let Some(ref dest) = self.dest {
            if !self.buffer.is_empty() {
                let mut inserts = std::mem::take(&mut self.buffer);
                inserts.push(Insert::new("//seq", msg.seq().to_be_bytes().to_vec()));
                let batch = InsertBatch::new(inserts);
                dest.do_send(batch);
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
