// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use e3_events::{EnclaveEvent, EventLog, Unsequenced};

pub struct InMemEventLog {
    log: Vec<EnclaveEvent<Unsequenced>>,
}

impl InMemEventLog {
    pub fn new() -> Self {
        Self { log: Vec::new() }
    }
}

impl EventLog for InMemEventLog {
    fn read_from(
        &self,
        from: u64,
    ) -> Box<dyn Iterator<Item = Result<(u64, EnclaveEvent<Unsequenced>)>>> {
        // Convert 1-indexed sequence to 0-indexed array position
        let start_idx = from.saturating_sub(1) as usize;

        let events: Vec<_> = self
            .log
            .iter()
            .skip(start_idx)
            .enumerate()
            .map(|(i, event)| Ok((from + i as u64, event.clone())))
            .collect();

        Box::new(events.into_iter())
    }
    fn append(&mut self, event: &EnclaveEvent<Unsequenced>) -> Result<u64> {
        self.log.push(event.to_owned());
        Ok(self.log.len() as u64)
    }
}
