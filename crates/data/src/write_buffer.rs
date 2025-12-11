// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Actor, Handler, Message, Recipient};
use e3_events::CommitSnapshot;

use crate::{Insert, InsertBatch};

pub struct WriteBuffer {
    dest: Option<Recipient<InsertBatch>>,
    buffer: Vec<Insert>,
}

impl Actor for WriteBuffer {
    type Context = actix::Context<Self>;
}

impl WriteBuffer {
    pub fn new() -> Self {
        Self {
            dest: None,
            buffer: Vec::new(),
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
        self.buffer.push(msg);
    }
}

impl Handler<CommitSnapshot> for WriteBuffer {
    type Result = ();

    fn handle(&mut self, _: CommitSnapshot, _: &mut Self::Context) -> Self::Result {
        if let Some(ref dest) = self.dest {
            if !self.buffer.is_empty() {
                let inserts = std::mem::take(&mut self.buffer);
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
