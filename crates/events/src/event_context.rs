// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::{fmt, ops::Deref};

use serde::{Deserialize, Serialize};

use crate::{
    E3id, EventContextAccessors, EventContextSeq, EventId, SeqState, Sequenced, Unsequenced,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AggregateId(usize);

impl AggregateId {
    pub fn new(value: usize) -> Self {
        Self(value)
    }
}

impl From<Option<E3id>> for AggregateId {
    fn from(value: Option<E3id>) -> Self {
        if let Some(e3_id) = value {
            Self::new(e3_id.chain_id() as usize)
        } else {
            Self::new(0)
        }
    }
}

impl Deref for AggregateId {
    type Target = usize;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for AggregateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EventContext<S: SeqState> {
    id: EventId,
    causation_id: EventId,
    origin_id: EventId,
    seq: S::Seq,
    ts: u128,
}

impl EventContext<Sequenced> {
    /// Tracks events as they create other events
    pub fn causes(self, id: EventId, ts: u128) -> EventContext<Unsequenced> {
        EventContext::<Unsequenced>::new(id, self.id, self.origin_id, ts)
    }
}

impl EventContext<Unsequenced> {
    pub fn new(id: EventId, causation_id: EventId, origin_id: EventId, ts: u128) -> Self {
        Self {
            id,
            causation_id,
            origin_id,
            seq: (),
            ts,
        }
    }

    pub fn sequence(self, value: u64) -> EventContext<Sequenced> {
        EventContext::<Sequenced> {
            seq: value,
            id: self.id,
            causation_id: self.causation_id,
            origin_id: self.origin_id,
            ts: self.ts,
        }
    }
}

impl<S: SeqState> EventContextAccessors for EventContext<S> {
    fn id(&self) -> EventId {
        self.id
    }

    fn causation_id(&self) -> EventId {
        self.causation_id
    }

    fn origin_id(&self) -> EventId {
        self.origin_id
    }

    fn ts(&self) -> u128 {
        self.ts
    }
}

impl EventContextSeq for EventContext<Sequenced> {
    fn seq(&self) -> u64 {
        self.seq
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        event_context::{AggregateId, EventContext},
        EventId,
    };

    #[test]
    fn test_event_context_cycle() {
        let mut events = vec![];

        let one =
            EventContext::new(EventId::hash(1), EventId::hash(1), EventId::hash(1), 1).sequence(1);
        events.push(one.clone());

        let two = one.causes(EventId::hash(2), 2).sequence(2);
        events.push(two.clone());

        let three = two.causes(EventId::hash(3), 3).sequence(3);
        events.push(three.clone());

        assert_eq!(
            events,
            vec![
                EventContext {
                    seq: 1,
                    id: EventId::hash(1),
                    origin_id: EventId::hash(1),
                    causation_id: EventId::hash(1),
                    ts: 1,
                },
                EventContext {
                    seq: 2,
                    id: EventId::hash(2),
                    origin_id: EventId::hash(1),
                    causation_id: EventId::hash(1),
                    ts: 2,
                },
                EventContext {
                    seq: 3,
                    id: EventId::hash(3),
                    origin_id: EventId::hash(1),
                    causation_id: EventId::hash(2),
                    ts: 3,
                },
            ]
        )
    }
}
