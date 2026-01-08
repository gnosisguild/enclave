// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::{fmt, ops::Deref};

use serde::{Deserialize, Serialize};

use crate::{E3id, EventContext, EventContextSeq, EventId, SeqState, Sequenced, Unsequenced};

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
pub struct ConcreteEventContext<S: SeqState> {
    id: EventId,
    causation_id: EventId,
    origin_id: EventId,
    aggregate_id: AggregateId,
    seq: S::Seq,
    ts: u128,
}

impl ConcreteEventContext<Sequenced> {
    /// Tracks events as they create other events
    pub fn causes(
        self,
        id: EventId,
        aggregate_id: AggregateId,
        ts: u128,
    ) -> ConcreteEventContext<Unsequenced> {
        ConcreteEventContext::<Unsequenced>::new(id, self.id, self.origin_id, aggregate_id, ts)
    }
}

impl ConcreteEventContext<Unsequenced> {
    pub fn new(
        id: EventId,
        causation_id: EventId,
        origin_id: EventId,
        aggregate_id: AggregateId,
        ts: u128,
    ) -> Self {
        Self {
            id,
            causation_id,
            origin_id,
            aggregate_id,
            seq: (),
            ts,
        }
    }

    pub fn sequence(self, value: u64) -> ConcreteEventContext<Sequenced> {
        ConcreteEventContext::<Sequenced> {
            seq: value,
            id: self.id,
            causation_id: self.causation_id,
            origin_id: self.origin_id,
            aggregate_id: self.aggregate_id,
            ts: self.ts,
        }
    }
}

impl<S: SeqState> EventContext for ConcreteEventContext<S> {
    fn id(&self) -> EventId {
        self.id
    }

    fn causation_id(&self) -> EventId {
        self.causation_id
    }

    fn origin_id(&self) -> EventId {
        self.origin_id
    }

    fn aggregate_id(&self) -> AggregateId {
        self.aggregate_id
    }

    fn ts(&self) -> u128 {
        self.ts
    }
}

impl EventContextSeq for ConcreteEventContext<Sequenced> {
    fn seq(&self) -> u64 {
        self.seq
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        event_context::{AggregateId, ConcreteEventContext},
        EventId,
    };

    #[test]
    fn test_event_context_cycle() {
        let mut events = vec![];

        let one = ConcreteEventContext::new(
            EventId::hash(1),
            EventId::hash(1),
            EventId::hash(1),
            AggregateId::new(1),
            1,
        )
        .sequence(1);
        events.push(one.clone());

        let two = one
            .causes(EventId::hash(2), AggregateId::new(1), 2)
            .sequence(2);
        events.push(two.clone());

        let three = two
            .causes(EventId::hash(3), AggregateId::new(2), 3)
            .sequence(3);
        events.push(three.clone());

        assert_eq!(
            events,
            vec![
                ConcreteEventContext {
                    aggregate_id: AggregateId::new(1),
                    seq: 1,
                    id: EventId::hash(1),
                    origin_id: EventId::hash(1),
                    causation_id: EventId::hash(1),
                    ts: 1,
                },
                ConcreteEventContext {
                    aggregate_id: AggregateId::new(1),
                    seq: 2,
                    id: EventId::hash(2),
                    origin_id: EventId::hash(1),
                    causation_id: EventId::hash(1),
                    ts: 2,
                },
                ConcreteEventContext {
                    aggregate_id: AggregateId::new(2),
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
