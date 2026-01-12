// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Actor, Handler, Message, Recipient};
use e3_events::hlc::HlcTimestamp;
use e3_events::{AggregateId, CommitSnapshot, EventContextAccessors};
use std::{
    collections::HashMap,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{Insert, InsertBatch};

#[derive(Debug)]
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

fn process_expired_inserts(
    aggregate_buffers: &HashMap<AggregateId, AggregateBuffer>,
    config: &HashMap<AggregateId, u64>,
    now: u64,
) -> (HashMap<AggregateId, AggregateBuffer>, Vec<Insert>) {
    let mut updated_buffers = HashMap::new();
    let mut all_expired_inserts = Vec::new();

    for (aggregate_id, agg_buffer) in aggregate_buffers {
        let delay_micros = config.get(aggregate_id).copied().unwrap_or(0);
        let cutoff_time = now.saturating_sub(delay_micros);
        let mut expired_inserts = Vec::new();
        let mut remaining_inserts = Vec::new();

        for insert in &agg_buffer.buffer {
            if let Some(ctx) = insert.ctx() {
                let event_wall_time = HlcTimestamp::wall_time(ctx.ts());
                if event_wall_time < cutoff_time {
                    expired_inserts.push(insert.clone());
                } else {
                    remaining_inserts.push(insert.clone());
                }
            } else {
                // If there is no context just flush it
                expired_inserts.push(insert.clone());
            }
        }

        all_expired_inserts.extend(expired_inserts);

        if !remaining_inserts.is_empty() {
            let mut new_agg_buffer = AggregateBuffer::new();
            new_agg_buffer.buffer = remaining_inserts;
            updated_buffers.insert(aggregate_id.clone(), new_agg_buffer);
        }
    }

    (updated_buffers, all_expired_inserts)
}

impl Handler<CommitSnapshot> for WriteBuffer {
    type Result = ();

    fn handle(&mut self, _msg: CommitSnapshot, _: &mut Self::Context) -> Self::Result {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_secs(0))
            .as_micros() as u64;

        if let Some(ref dest) = self.dest {
            let (updated_buffers, expired_inserts) =
                process_expired_inserts(&self.aggregate_buffers, &self.config, now);
            if !expired_inserts.is_empty() {
                let batch = InsertBatch::new(expired_inserts);
                dest.do_send(batch);
            }

            self.aggregate_buffers = updated_buffers;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::Insert;
    use e3_events::{hlc::HlcTimestamp, EventContext, EventId};

    #[test]
    fn test_process_expired_inserts() {
        let aggregate_id = AggregateId::new(1);

        // Create test inserts with different timestamps (in microseconds)
        // Create proper HlcTimestamps and encode them to u128
        let old_hlc = HlcTimestamp::new(500_000, 0, 1); // 0.5 seconds ago
        let new_hlc = HlcTimestamp::new(3_000_000, 0, 2); // 3 seconds from epoch

        let old_ctx = EventContext::new(
            EventId::hash(1),
            EventId::hash(1),
            EventId::hash(1),
            old_hlc.into(),
            aggregate_id.clone(),
        )
        .sequence(1);

        let new_ctx = EventContext::new(
            EventId::hash(2),
            EventId::hash(2),
            EventId::hash(2),
            new_hlc.into(),
            aggregate_id.clone(),
        )
        .sequence(2);

        let old_insert = Insert::new_with_context("old_key", b"old_value".to_vec(), old_ctx);
        let new_insert = Insert::new_with_context("new_key", b"new_value".to_vec(), new_ctx);
        let insert_no_ctx = Insert::new("no_ctx_key", b"no_ctx_value".to_vec());

        // Set up aggregate buffer with mixed inserts
        let mut agg_buffer = AggregateBuffer::new();
        agg_buffer.buffer.push(old_insert.clone());
        agg_buffer.buffer.push(new_insert.clone());
        agg_buffer.buffer.push(insert_no_ctx.clone());

        let mut aggregate_buffers = HashMap::new();
        aggregate_buffers.insert(aggregate_id.clone(), agg_buffer);

        // Set config with 1 second delay
        let mut config = HashMap::new();
        config.insert(aggregate_id.clone(), 1_000_000); // 1 second in microseconds

        // Use current time of 2 seconds, so old insert (0.5s) should expire,
        // new insert (3s) and insert without context should remain
        let now = 2_000_000; // 2 seconds in microseconds

        let (updated_buffers, expired_inserts) =
            process_expired_inserts(&aggregate_buffers, &config, now);

        // Verify expired inserts
        assert_eq!(expired_inserts.len(), 1);
        assert_eq!(expired_inserts[0], old_insert);

        // Verify remaining inserts in buffer
        assert_eq!(updated_buffers.len(), 1);
        let remaining_buffer = updated_buffers.get(&aggregate_id).unwrap();
        assert_eq!(remaining_buffer.buffer.len(), 2);
        assert!(remaining_buffer.buffer.contains(&new_insert));
        assert!(remaining_buffer.buffer.contains(&insert_no_ctx));
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
