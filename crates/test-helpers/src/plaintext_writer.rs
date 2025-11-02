// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::path::PathBuf;

use super::write_file_with_dirs;
use actix::{Actor, Addr, Handler};
use anyhow::{Context, Result};
use e3_events::{EnclaveEvent, EventBus, PlaintextAggregated, Subscribe};
use tracing::{error, info};

pub struct PlaintextWriter {
    path: PathBuf,
    experimental_trbfv: bool,
}

impl PlaintextWriter {
    pub fn attach(
        path: &PathBuf,
        bus: Addr<EventBus<EnclaveEvent>>,
        experimental_trbfv: bool,
    ) -> Addr<Self> {
        let addr = Self {
            path: path.to_owned(),
            experimental_trbfv,
        }
        .start();
        bus.do_send(Subscribe {
            listener: addr.clone().recipient(),
            event_type: "PlaintextAggregated".to_string(),
        });
        addr
    }

    fn handle_trbfv_plaintext_aggregation(
        &mut self,
        data: &PlaintextAggregated,
    ) -> Result<Vec<u64>> {
        // Each bytes in plaintext is a [u64] corresponding to the ciphertext
        // That is the output of the program here we decrypt extracting the first u64
        let results = data
            .decrypted_output
            .iter()
            .map(|a| {
                bincode::deserialize::<Vec<u64>>(&a.extract_bytes())
                    .context("Could not deserialize plaintext")?
                    .first()
                    .copied()
                    .context("Vector was empty")
            })
            .collect::<Result<Vec<u64>>>()?;
        Ok(results)
    }

    fn handle_plaintext_aggregation(&mut self, data: &PlaintextAggregated) -> Result<Vec<u64>> {
        // HACK: decrypted output will be an array of ArcBytes and we will use this moving forward. For now
        // only having the plaintext writer compatible with legacy tests and extracting the first value
        let Some(decrypted) = data.decrypted_output.first() else {
            error!("Decrypted output must not be empty!");
            return Ok(vec![]);
        };
        let output: Vec<u64> = decrypted
            .chunks_exact(8)
            .map(|chunk| u64::from_le_bytes(chunk.try_into().unwrap()))
            .collect();

        Ok(output)
    }

    pub fn handle_plaintext_aggregated(&mut self, msg: PlaintextAggregated) -> Result<()> {
        let output = if !self.experimental_trbfv {
            self.handle_plaintext_aggregation(&msg)?
        } else {
            self.handle_trbfv_plaintext_aggregation(&msg)?
        };
        let contents: Vec<String> = output.iter().map(|&num| num.to_string()).collect();

        info!(path = ?&self.path, "Writing Plaintext To Path");
        write_file_with_dirs(&self.path, format!("{}", contents.join(",")).as_bytes()).unwrap();

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
