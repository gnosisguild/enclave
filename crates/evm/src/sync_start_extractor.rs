// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Actor, Addr, Handler, Recipient};
use e3_events::{EnclaveEvent, EnclaveEventData, Event, SyncStart};

pub struct SyncStartExtractor {
    dest: Recipient<SyncStart>,
}

impl SyncStartExtractor {
    pub fn new(dest: impl Into<Recipient<SyncStart>>) -> Self {
        Self { dest: dest.into() }
    }

    pub fn setup(dest: impl Into<Recipient<SyncStart>>) -> Addr<Self> {
        Self::new(dest).start()
    }
}
impl Actor for SyncStartExtractor {
    type Context = actix::Context<Self>;
}

impl Handler<EnclaveEvent> for SyncStartExtractor {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        if let EnclaveEventData::SyncStart(evt) = msg.into_data() {
            self.dest.do_send(evt)
        }
    }
}
