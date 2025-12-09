// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Actor, Handler};
use e3_events::sequencer;
use tracing::error;

use crate::{AppendOnlyStore, Insert, SeekableStore};

pub struct EventStore<L, S>
where
    L: AppendOnlyStore,
    S: SeekableStore,
{
    event_log: L,
    hlc_store: S,
}

// impl EventStore {
//     pub fn in_mem() -> EventStore<InMemCommitLog, InMemDb> {
//         Self {
//             event_log: InMemCommitLog::new(),
//             hlc_store: InMemDb::new(),
//         }
//     }
//
//     pub fn new(log: EventLog, db: SledDb) -> EventStore<EventLog, SledDb> {
//         Self {
//             event_log: log,
//             hlc_store: db,
//         }
//     }
// }

impl<L, S> Actor for EventStore<L, S>
where
    L: AppendOnlyStore + 'static,
    S: SeekableStore + 'static,
{
    type Context = actix::Context<Self>;
}

impl<L, S> Handler<sequencer::PersistRequest> for EventStore<L, S>
where
    L: AppendOnlyStore + 'static,
    S: SeekableStore + 'static,
{
    type Result = ();
    fn handle(&mut self, msg: sequencer::PersistRequest, _: &mut Self::Context) -> Self::Result {
        match {
            let event = msg.event;
            let sender = msg.sender;
            let ts = event.get_ts();
            let seq = self.event_log.append_msg(event.to_bytes()?)?;
            self.hlc_store
                .insert(Insert::new(ts, seq.to_be_bytes().to_vec()))?;
            sender.do_send(sequencer::EventPersisted { seq, event });
            Ok(())
        } {
            Ok(_) => (),
            Err(e) => error!(""),
        }
    }
}
