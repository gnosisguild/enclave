// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::path::PathBuf;

use super::write_file_with_dirs;
use actix::{Actor, Addr, Context, Handler};
use e3_events::{EnclaveEvent, EventBus, Subscribe};
use tracing::{error, info};

pub struct PlaintextWriter {
    path: PathBuf,
}

impl PlaintextWriter {
    pub fn attach(path: &PathBuf, bus: Addr<EventBus<EnclaveEvent>>) -> Addr<Self> {
        let addr = Self {
            path: path.to_owned(),
        }
        .start();
        bus.do_send(Subscribe {
            listener: addr.clone().recipient(),
            event_type: "PlaintextAggregated".to_string(),
        });
        addr
    }
}

impl Actor for PlaintextWriter {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for PlaintextWriter {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, _: &mut Self::Context) -> Self::Result {
        if let EnclaveEvent::PlaintextAggregated { data, .. } = msg.clone() {
            // HACK: decrypted output will be an array of ArcBytes and we will use this moving forward. For now
            // only having the plaintext writer compatible with legacy tests and extracting the first value
            let Some(decrypted) = data.decrypted_output.first() else {
                error!("Decrypted output must not be empty!");
                return;
            };
            let output: Vec<u64> = decrypted
                .chunks_exact(8)
                .map(|chunk| u64::from_le_bytes(chunk.try_into().unwrap()))
                .collect();

            info!(path = ?&self.path, "Writing Plaintext To Path");
            let contents: Vec<String> = output.iter().map(|&num| num.to_string()).collect();

            write_file_with_dirs(&self.path, format!("{}", contents.join(",")).as_bytes()).unwrap();
        }
    }
}
