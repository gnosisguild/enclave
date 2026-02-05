// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
use std::mem::replace;

use actix::{Actor, ActorContext, Addr, AsyncContext, Handler, Message, Recipient};
use tracing::debug;

use crate::{trap, Die, EType, Insert, InsertBatch, PanicDispatcher};

#[derive(Message)]
#[rtype(result = "()")]
pub struct Flush;

pub struct Batch {
    inserts: Vec<Insert>,
    db: Recipient<InsertBatch>,
}

impl Batch {
    pub fn new(db: impl Into<Recipient<InsertBatch>>, inserts: Vec<Insert>) -> Self {
        Self {
            inserts,
            db: db.into(),
        }
    }
    pub fn spawn(db: impl Into<Recipient<InsertBatch>>, inserts: Vec<Insert>) -> Addr<Self> {
        Self::new(db, inserts).start()
    }
}

impl Actor for Batch {
    type Context = actix::Context<Self>;
}

impl Handler<Insert> for Batch {
    type Result = ();
    fn handle(&mut self, msg: Insert, _: &mut Self::Context) -> Self::Result {
        self.inserts.push(msg)
    }
}

impl Handler<Flush> for Batch {
    type Result = ();
    fn handle(&mut self, _: Flush, ctx: &mut Self::Context) -> Self::Result {
        let inserts = replace(&mut self.inserts, Vec::new());
        trap(EType::IO, &PanicDispatcher::new(), || {
            if inserts.len() > 0 {
                self.db.try_send(InsertBatch::new(inserts))?;
            }
            ctx.notify(Die);
            Ok(())
        })
    }
}

impl Handler<Die> for Batch {
    type Result = ();
    fn handle(&mut self, _: Die, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop();
    }
}
