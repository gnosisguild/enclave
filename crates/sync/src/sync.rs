// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Actor, Handler, Message};

struct Sync;

impl Sync {}

impl Actor for Sync {
    type Context = actix::Context<Self>;
}

impl Handler<Bootstrap> for Sync {
    type Result = ();
    fn handle(&mut self, msg: Bootstrap, ctx: &mut Self::Context) -> Self::Result {
        // Publish SyncStart
    }
}

#[derive(Message)]
#[rtype("()")]
pub struct Bootstrap;
