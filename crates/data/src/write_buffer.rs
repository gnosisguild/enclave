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
}

impl Actor for WriteBuffer {
    type Context = actix::Context<Self>;
}

impl WriteBuffer {
    pub fn new() -> Self {
        Self { dest: None }
    }
}

impl Handler<ForwardTo> for WriteBuffer {
    type Result = ();
    fn handle(&mut self, msg: ForwardTo, ctx: &mut Self::Context) -> Self::Result {
        self.dest = Some(msg.dest())
    }
}

impl Handler<Insert> for WriteBuffer {
    type Result = ();
    fn handle(&mut self, msg: Insert, ctx: &mut Self::Context) -> Self::Result {
        // store insert in buffer
    }
}

impl Handler<CommitSnapshot> for WriteBuffer {
    type Result = ();
    fn handle(&mut self, msg: CommitSnapshot, ctx: &mut Self::Context) -> Self::Result {
        // send all inserts to
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
