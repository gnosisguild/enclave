// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Actor, Handler, Message, Recipient};
use commitlog::CommitLog;
use e3_events::{trap, BusHandle, EType, EnclaveEvent};

use crate::{Insert, KeyValStore, SledDb};

#[derive(Message)]
#[rtype("()")]
pub struct PersistRequest {
    pub event: EnclaveEvent,
    pub sender: Recipient<EventPersisted>,
}

#[derive(Message)]
#[rtype("()")]
pub struct EventPersisted(pub u64);

pub struct EventStore {
    hlc_store: SledDb,
    event_log: CommitLog,
    bus: BusHandle,
}

impl Actor for EventStore {
    type Context = actix::Context<Self>;
}

impl Handler<PersistRequest> for EventStore {
    type Result = ();
    fn handle(&mut self, msg: PersistRequest, _: &mut Self::Context) -> Self::Result {
        trap(EType::Data, &self.bus, || {
            let event = msg.event;
            let sender = msg.sender;
            let ts = event.get_ts();
            let seq = self.event_log.append_msg(event.to_bytes()?)?;
            self.hlc_store
                .insert(Insert::new(ts, seq.to_be_bytes().to_vec()));
            sender.try_send(EventPersisted(seq))?;
            Ok(())
        })
    }
}
