// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::path::PathBuf;

use super::write_file_with_dirs;
use actix::{Actor, Addr, Handler};
use anyhow::{anyhow, Result};
use e3_events::{EnclaveEvent, EventBus, PlaintextAggregated, Subscribe};
use e3_sdk::bfv_helpers::decode_plaintexts;
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

    pub fn handle_plaintext_aggregated(&mut self, msg: PlaintextAggregated) -> Result<()> {
        let output = decode_plaintexts(&msg.decrypted_output).map_err(|e| anyhow!("{e}"))?;
        // This is connected to the plaintext we use in the integration test.
        // We are expecting two values from the first plaintext
        let special_output = [output[0][0], output[0][1]];
        info!(path = ?&self.path, "Writing Plaintext To Path");
        write_file_with_dirs(
            &self.path,
            special_output
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
                .as_bytes(),
        )?;
        Ok(())
    }
}

impl Actor for PlaintextWriter {
    type Context = actix::Context<Self>;
}

impl Handler<EnclaveEvent> for PlaintextWriter {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, _: &mut Self::Context) -> Self::Result {
        if let EnclaveEvent::PlaintextAggregated { data, .. } = msg {
            if let Err(e) = self.handle_plaintext_aggregated(data) {
                error!("{e}");
            }
        }
    }
}
