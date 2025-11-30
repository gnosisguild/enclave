// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::path::PathBuf;

use super::write_file_with_dirs;
use actix::{Actor, Addr, Context, Handler};
use e3_events::{EnclaveEvent, EnclaveEventData, EventBus, EventManager, Subscribe};
use tracing::info;

pub struct PublicKeyWriter {
    path: PathBuf,
}

impl PublicKeyWriter {
    pub fn attach(path: &PathBuf, bus: EventManager<EnclaveEvent>) -> Addr<Self> {
        let addr = Self {
            path: path.to_owned(),
        }
        .start();
        bus.do_send(Subscribe {
            listener: addr.clone().recipient(),
            event_type: "PublicKeyAggregated".to_string(),
        });
        addr
    }
}

impl Actor for PublicKeyWriter {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for PublicKeyWriter {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, _: &mut Self::Context) -> Self::Result {
        if let EnclaveEventData::PublicKeyAggregated(data) = msg.into_data() {
            info!(path = ?&self.path, "Writing Pubkey To Path");
            write_file_with_dirs(&self.path, &data.pubkey).unwrap();
        }
    }
}
