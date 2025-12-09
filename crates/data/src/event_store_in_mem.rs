// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Actor, Handler};
use e3_events::{sequencer, trap, BusHandle, EType};

use crate::{InMemCommitLog, InMemDb, Insert, KeyValStore};

pub struct InMemEventStore {
    hlc_store: InMemDb,
    event_log: InMemCommitLog,
    bus: BusHandle,
}

impl Actor for InMemEventStore {
    type Context = actix::Context<Self>;
}

impl Handler<sequencer::PersistRequest> for InMemEventStore {
    type Result = ();
    fn handle(&mut self, msg: PersistRequest, _: &mut Self::Context) -> Self::Result {
        trap(EType::Data, &self.bus, || {
            let event = msg.event;
            let sender = msg.sender;
            let ts = event.get_ts();
            let seq = self.event_log.append_msg(event.to_bytes()?)?;
            self.hlc_store
                .insert(Insert::new(ts, seq.to_be_bytes().to_vec()))?;
            sender.try_send(sequencer::EventPersisted(seq))?;
            Ok(())
        })
    }
}
