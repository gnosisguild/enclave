// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::traits::EventContextSeq;

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum SeqCursor {
    Done,
    Next(u64),
}

pub fn compute_seq_cursor<T: EventContextSeq>(events: &[T], limit: usize) -> SeqCursor {
    if events.len() == limit {
        let last_seq = events.last().map(|e| e.seq()).unwrap_or(0);
        SeqCursor::Next(last_seq)
    } else {
        SeqCursor::Done
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::EventContextSeq;

    struct MockEvent(u64);
    impl EventContextSeq for MockEvent {
        fn seq(&self) -> u64 {
            self.0
        }
    }

    #[test]
    fn test_seq_cursor_done_when_under_limit() {
        let events = vec![MockEvent(1), MockEvent(2)];
        let cursor = compute_seq_cursor(&events, 10);
        assert!(matches!(cursor, SeqCursor::Done));
    }

    #[test]
    fn test_seq_cursor_next_when_at_limit() {
        let events = vec![MockEvent(1), MockEvent(2)];
        let cursor = compute_seq_cursor(&events, 2);
        match cursor {
            SeqCursor::Next(seq) => assert_eq!(seq, 2),
            SeqCursor::Done => panic!("Expected Next, got Done"),
        }
    }

    #[test]
    fn test_seq_cursor_uses_last_event_seq() {
        let events = vec![MockEvent(100), MockEvent(200), MockEvent(300)];
        let cursor = compute_seq_cursor(&events, 3);
        match cursor {
            SeqCursor::Next(seq) => assert_eq!(seq, 300),
            SeqCursor::Done => panic!("Expected Next, got Done"),
        }
    }

    #[test]
    fn test_seq_cursor_empty_returns_done() {
        let events: Vec<MockEvent> = vec![];
        let cursor = compute_seq_cursor(&events, 10);
        assert!(matches!(cursor, SeqCursor::Done));
    }

    #[test]
    fn test_seq_cursor_single_event_at_limit() {
        let events = vec![MockEvent(42)];
        let cursor = compute_seq_cursor(&events, 1);
        match cursor {
            SeqCursor::Next(seq) => assert_eq!(seq, 42),
            SeqCursor::Done => panic!("Expected Next, got Done"),
        }
    }
}
