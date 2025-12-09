use crate::AppendOnlyStore;
use anyhow::Result;
use commitlog::{CommitLog, LogOptions};
use std::path::PathBuf;

pub struct EventLog(CommitLog);

impl EventLog {
    pub fn new(path: &PathBuf) -> Self {
        let opts = LogOptions::new(path);
        let log = CommitLog::new(opts);
        Self(log)
    }
}

impl AppendOnlyStore for EventLog {
    fn append_msg(&mut self, payload: Vec<u8>) -> Result<u64> {
        self.0.append_msg(payload)
    }
}
