// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use e3_events::{EnclaveEvent, EventLog, Unsequenced};

pub struct InMemEventLog;

impl InMemEventLog {
    pub fn new() -> Self {
        Self {}
    }
}

impl EventLog for InMemEventLog {
    fn read_from(
        &self,
        from: u64,
    ) -> Box<dyn Iterator<Item = Result<(u64, EnclaveEvent<Unsequenced>)>>> {
        Box::new(vec![].into_iter())
    }
    fn append(&mut self, event: &EnclaveEvent<Unsequenced>) -> Result<u64> {
        Ok(1u64)
    }
}
