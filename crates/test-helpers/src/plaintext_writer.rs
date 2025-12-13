// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::path::PathBuf;

use super::write_file_with_dirs;
use actix::{Actor, Addr, Context, Handler};
use e3_events::{prelude::*, BusHandle, EnclaveEvent, EnclaveEventData};
use e3_sdk::bfv_helpers::decode_bytes_to_vec_u64;
use tracing::{error, info};

pub struct PlaintextWriter {
    path: PathBuf,
}

impl PlaintextWriter {
    pub fn attach(path: &PathBuf, bus: BusHandle) -> Addr<Self> {
        let addr = Self {
            path: path.to_owned(),
        }
        .start();
        bus.subscribe("PlaintextAggregated", addr.clone().recipient());
        addr
    }
}

impl Actor for PlaintextWriter {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for PlaintextWriter {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, _: &mut Self::Context) -> Self::Result {
        if let EnclaveEventData::PlaintextAggregated(data) = msg.into_data() {
            let Some(decrypted) = data.decrypted_output.first() else {
                error!("Decrypted output must not be empty!");
                return;
            };

            let output = decode_bytes_to_vec_u64(&decrypted).unwrap();

            info!(path = ?&self.path, "Writing Plaintext To Path");
            let contents: Vec<String> = output.iter().map(|&num| num.to_string()).collect();

            write_file_with_dirs(&self.path, format!("{}", contents.join(",")).as_bytes()).unwrap();
        }
    }
}
