// SPDX-License-Identifier: LGPL-2.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Context;
use anyhow::Result;
use commitlog::message::MessageSet;
use commitlog::{CommitLog, LogOptions, ReadLimit};
use e3_events::{EnclaveEvent, EventLog, Unsequenced};
use std::path::PathBuf;
use tracing::error;

pub struct CommitLogEventLog {
    log: CommitLog,
}

impl CommitLogEventLog {
    /// Creates a new CommitLogEventLog backed by a CommitLog stored at the given path.
    ///
    /// The created log uses configured CommitLog options (including a permissive
    /// maximum message size) and returns an error if the underlying CommitLog
    /// cannot be opened or created.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::PathBuf;
    /// let dir = tempfile::tempdir().unwrap();
    /// let log = CommitLogEventLog::new(&dir.path().to_path_buf()).unwrap();
    /// ```
    pub fn new(path: &PathBuf) -> Result<Self> {
        let mut opts = LogOptions::new(path);
        // TODO: drive this from config - currently set high to be permissive
        opts.message_max_bytes(32 * 1024 * 1024);
        let log = CommitLog::new(opts)?;
        Ok(Self { log })
    }
}

impl EventLog for CommitLogEventLog {
    /// Appends an EnclaveEvent to the underlying commit log and returns its 1-indexed sequence number.
    ///
    /// The function serializes the provided event and stores it in the commit log.
    ///
    /// # Returns
    ///
    /// `u64` containing the 1-indexed sequence number assigned to the appended event.
    ///
    /// # Examples
    ///
    /// ```
    /// // assuming `log` is a mutable CommitLogEventLog and `event` is an EnclaveEvent<Unsequenced>
    /// let seq = log.append(&event).unwrap();
    /// assert!(seq >= 1);
    /// ```
    fn append(&mut self, event: &EnclaveEvent<Unsequenced>) -> Result<u64> {
        let bytes = bincode::serialize(event)?;
        let offset = self
            .log
            .append_msg(&bytes)
            .context("Failed to append to event log")?;
        // Return 1-indexed sequence number
        Ok(offset + 1)
    }

    /// Reads events starting at a 1-indexed sequence number and returns an iterator over (sequence, event) pairs.
    ///
    /// The `from` parameter is a 1-indexed sequence number; reading begins from that sequence (or the start when `from` is 0).
    /// The returned iterator yields tuples where the first element is the 1-indexed sequence number and the second is the deserialized `EnclaveEvent<Unsequenced>`.
    /// Messages that fail deserialization are skipped.
    ///
    /// # Examples
    ///
    /// ```
    /// # use e3_events::{EnclaveEvent, Unsequenced};
    /// # use tempfile::tempdir;
    /// # use std::path::PathBuf;
    /// # // setup omitted: create CommitLogEventLog and append events
    /// # let dir = tempdir().unwrap();
    /// # let path = dir.path().to_path_buf();
    /// # let mut log = CommitLogEventLog::new(&path).unwrap();
    /// # let e = EnclaveEvent::<Unsequenced>::new(vec![1u8], 0);
    /// # log.append(&e).unwrap();
    /// let mut iter = log.read_from(1);
    /// let results: Vec<(u64, EnclaveEvent<Unsequenced>)> = iter.collect();
    /// assert!(!results.is_empty());
    /// assert_eq!(results[0].0, 1);
    /// ```
    fn read_from(&self, from: u64) -> Box<dyn Iterator<Item = (u64, EnclaveEvent<Unsequenced>)>> {
        // Convert 1-indexed sequence to 0-indexed offset
        let mut current_offset = from.saturating_sub(1);
        let mut events = Vec::new();

        loop {
            let message_buf = match self.log.read(current_offset, ReadLimit::default()) {
                Ok(msgs) => msgs,
                Err(_) => break,
            };

            let mut count = 0;
            for msg in message_buf.iter() {
                if let Ok(event) = bincode::deserialize::<EnclaveEvent<Unsequenced>>(msg.payload())
                {
                    // Convert 0-indexed offset back to 1-indexed sequence number
                    events.push((msg.offset() + 1, event));
                    current_offset = msg.offset() + 1; // Next offset to read from
                } else {
                    error!("Error deserializing event in read_from... skipping");
                }
                count += 1;
            }

            // No more messages to read
            if count == 0 {
                break;
            }
        }

        Box::new(events.into_iter())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use e3_events::{EnclaveEventData, EventConstructorWithTimestamp, TestEvent};
    use tempfile::tempdir;

    fn event_from(data: impl Into<EnclaveEventData>) -> EnclaveEvent<Unsequenced> {
        EnclaveEvent::<Unsequenced>::new_with_timestamp(data.into().into(), 123)
    }

    #[test]
    fn test_append_and_read() {
        let dir = tempdir().unwrap();
        let mut log = CommitLogEventLog::new(&dir.path().to_path_buf()).unwrap();

        let event1 = event_from(TestEvent::new("one", 1));
        let event2 = event_from(TestEvent::new("two", 2));

        let offset1 = log.append(&event1).unwrap();
        let offset2 = log.append(&event2).unwrap();

        assert_eq!(offset1, 1); // 1-indexed
        assert_eq!(offset2, 2);

        // Read back from the beginning
        let events: Vec<_> = log.read_from(1).collect();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].0, 1);
        assert_eq!(events[1].0, 2);
    }

    #[test]
    fn test_read_from_offset() {
        let dir = tempdir().unwrap();
        let mut log = CommitLogEventLog::new(&dir.path().to_path_buf()).unwrap();

        let event1 = event_from(TestEvent::new("one", 1));
        let event2 = event_from(TestEvent::new("two", 2));
        let event3 = event_from(TestEvent::new("three", 3));

        log.append(&event1).unwrap();
        log.append(&event2).unwrap();
        log.append(&event3).unwrap();

        // Read from offset 2 (should get events 2 and 3)
        let events: Vec<_> = log.read_from(2).collect();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].0, 2);
        assert_eq!(events[1].0, 3);
    }

    #[test]
    fn test_read_empty_log() {
        let dir = tempdir().unwrap();
        let log = CommitLogEventLog::new(&dir.path().to_path_buf()).unwrap();

        let events: Vec<_> = log.read_from(1).collect();
        assert!(events.is_empty());
    }

    #[test]
    fn test_read_past_end() {
        let dir = tempdir().unwrap();
        let mut log = CommitLogEventLog::new(&dir.path().to_path_buf()).unwrap();

        let event = event_from(TestEvent::new("one", 1));
        log.append(&event).unwrap();

        // Read from offset beyond what exists
        let events: Vec<_> = log.read_from(100).collect();
        assert!(events.is_empty());
    }
}