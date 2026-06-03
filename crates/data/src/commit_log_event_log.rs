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

/// Maximum message size for both reads and writes (32 MB).
const MAX_MESSAGE_BYTES: usize = 32 * 1024 * 1024;

pub struct CommitLogEventLog {
    log: CommitLog,
}

impl CommitLogEventLog {
    pub fn new(path: &PathBuf) -> Result<Self> {
        let mut opts = LogOptions::new(path);
        // TODO: derive this from config - currently set high to be permissive
        opts.message_max_bytes(MAX_MESSAGE_BYTES);
        let log = CommitLog::new(opts)?;
        Ok(Self { log })
    }

    fn append_bytes(&mut self, bytes: &[u8]) -> Result<u64> {
        let offset = self
            .log
            .append_msg(&bytes)
            .context("Failed to append to event log")?;
        // Return 1-indexed sequence number
        Ok(offset + 1)
    }
}

impl EventLog for CommitLogEventLog {
    fn append(&mut self, event: &EnclaveEvent<Unsequenced>) -> Result<u64> {
        let bytes = bincode::serialize(event)?;
        self.append_bytes(&bytes)
    }

    fn read_from(&self, from: u64) -> Box<dyn Iterator<Item = (u64, EnclaveEvent<Unsequenced>)>> {
        // Convert 1-indexed sequence to 0-indexed offset
        let mut current_offset = from.saturating_sub(1);
        let mut events = Vec::new();
        // Sequence number of the first message that failed to deserialize, if any.
        // A deserialize failure is only tolerable when it is the *tail* of the log
        // (a torn write from a crash mid-append). If a corrupt entry is followed by
        // a valid one it is mid-log corruption and replaying past it would silently
        // diverge actor state, so we halt loudly instead of skipping.
        let mut corrupt_at: Option<u64> = None;

        loop {
            let message_buf = match self
                .log
                .read(current_offset, ReadLimit::max_bytes(MAX_MESSAGE_BYTES))
            {
                Ok(msgs) => msgs,
                Err(_) => break,
            };

            let mut count = 0;
            for msg in message_buf.iter() {
                let seq = msg.offset() + 1;
                match bincode::deserialize::<EnclaveEvent<Unsequenced>>(msg.payload()) {
                    Ok(event) => {
                        if let Some(bad_seq) = corrupt_at {
                            // We already saw a corrupt entry and now found a valid one
                            // after it: the corruption is NOT at the tail.
                            panic!(
                                "Non-tail corruption in event log: entry at seq {bad_seq} failed \
                                 to deserialize but is followed by a valid entry at seq {seq}. \
                                 Replaying past it would silently drop an event. Halting; operator \
                                 recovery required."
                            );
                        }
                        // Convert 0-indexed offset back to 1-indexed sequence number
                        events.push((seq, event));
                    }
                    Err(_) => {
                        // Defer the decision: tolerate only if nothing valid follows.
                        error!("Error deserializing event in read_from at seq {seq}");
                        if corrupt_at.is_none() {
                            corrupt_at = Some(seq);
                        }
                    }
                }
                current_offset = msg.offset() + 1; // Next offset to read from
                count += 1;
            }

            // No more messages to read
            if count == 0 {
                break;
            }
        }

        Box::new(events.into_iter())
    }

    fn head(&self) -> u64 {
        // `last_offset` is 0-indexed; convert to a 1-indexed sequence number.
        self.log.last_offset().map(|o| o + 1).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use e3_events::{EnclaveEventData, EventConstructorWithTimestamp, EventSource, TestEvent};
    use tempfile::tempdir;

    fn event_from(data: impl Into<EnclaveEventData>) -> EnclaveEvent<Unsequenced> {
        EnclaveEvent::<Unsequenced>::new_with_timestamp(
            data.into().into(),
            None,
            123,
            None,
            EventSource::Local,
        )
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
    fn test_read_from_corruption_at_end_causes_infinite_loop() {
        let dir = tempdir().unwrap();
        let mut log = CommitLogEventLog::new(&dir.path().to_path_buf()).unwrap();

        for i in 0..100 {
            let e = event_from(TestEvent::new("myevent", i));
            log.append(&e).unwrap();
        }
        // Corrupt the last message
        log.append_bytes(b"I am a bad event!").unwrap();

        // Ensure if last message is corrupt we don't end up in an infinite loop
        let _: Vec<_> = log.read_from(1).collect();
    }

    #[test]
    #[should_panic(expected = "Non-tail corruption")]
    fn test_read_from_non_tail_corruption_halts() {
        let dir = tempdir().unwrap();
        let mut log = CommitLogEventLog::new(&dir.path().to_path_buf()).unwrap();

        for i in 0..10 {
            let e = event_from(TestEvent::new("before", i));
            log.append(&e).unwrap();
        }
        // Corrupt entry in the MIDDLE of the log...
        log.append_bytes(b"I am a bad event!").unwrap();
        // ...followed by a valid entry, making the corruption non-tail.
        for i in 0..10 {
            let e = event_from(TestEvent::new("after", i));
            log.append(&e).unwrap();
        }

        let _: Vec<_> = log.read_from(1).collect();
    }

    #[test]
    fn test_head_reports_last_sequence() {
        let dir = tempdir().unwrap();
        let mut log = CommitLogEventLog::new(&dir.path().to_path_buf()).unwrap();
        assert_eq!(log.head(), 0);
        log.append(&event_from(TestEvent::new("one", 1))).unwrap();
        log.append(&event_from(TestEvent::new("two", 2))).unwrap();
        assert_eq!(log.head(), 2);
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
