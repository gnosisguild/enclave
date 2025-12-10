use std::path::PathBuf;

use anyhow::Result;
use commitlog::{CommitLog, LogOptions};
use e3_events::{EnclaveEvent, EventLog, Unsequenced};

pub struct CommitLogEventLog {
    log: CommitLog,
}

impl CommitLogEventLog {
    pub fn new(path: &PathBuf) -> Result<Self> {
        let opts = LogOptions::new(path);
        let log = CommitLog::new(opts)?;
        Ok(Self { log })
    }
}

impl EventLog for CommitLogEventLog {
    fn append(&mut self, event: &EnclaveEvent<Unsequenced>) -> Result<u64> {
        Ok(1u64)
    }

    fn read_from(
        &self,
        from: u64,
    ) -> Box<
        dyn Iterator<Item = std::result::Result<(u64, EnclaveEvent<Unsequenced>), anyhow::Error>>,
    > {
        Box::new(vec![].into_iter())
    }
}
