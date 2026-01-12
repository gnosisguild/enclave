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
    aggregate_id: AggregateId,
}

impl EventContext<Unsequenced> {
    pub fn new(
        id: EventId,
        causation_id: EventId,
        origin_id: EventId,
        ts: u128,
        aggregate_id: AggregateId,
    ) -> Self {
        Self {
            id,
            causation_id,
            origin_id,
            seq: (),
            ts,
            aggregate_id,
        }
    }

    pub fn new_origin(id: EventId, ts: u128, aggregate_id: AggregateId) -> Self {
        Self::new(id, id, id, ts, aggregate_id)
    }

    pub fn from_cause(
        id: EventId,
        cause: EventContext<Sequenced>,
        ts: u128,
        aggregate_id: AggregateId,
    ) -> Self {
        EventContext::new(id, cause.id(), cause.origin_id(), ts, aggregate_id)
    }

    pub fn sequence(self, value: u64) -> EventContext<Sequenced> {
        EventContext::<Sequenced> {
            seq: value,
            id: self.id,
            causation_id: self.causation_id,
            origin_id: self.origin_id,
            ts: self.ts,
            aggregate_id: self.aggregate_id,
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

    fn aggregate_id(&self) -> AggregateId {
        self.aggregate_id
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

        let one = EventContext::new(
            EventId::hash(1),
            EventId::hash(1),
            EventId::hash(1),
            1,
            AggregateId::new(1),
        )
        .sequence(1);
        events.push(one.clone());

        let two =
            EventContext::from_cause(EventId::hash(2), one, 2, AggregateId::new(1)).sequence(2);
        events.push(two.clone());

        let three =
            EventContext::from_cause(EventId::hash(3), two, 3, AggregateId::new(1)).sequence(3);
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
                    aggregate_id: AggregateId::new(1),
                },
                EventContext {
                    seq: 2,
                    id: EventId::hash(2),
                    origin_id: EventId::hash(1),
                    causation_id: EventId::hash(1),
                    ts: 2,
                    aggregate_id: AggregateId::new(1),
                },
                EventContext {
                    seq: 3,
                    id: EventId::hash(3),
                    origin_id: EventId::hash(1),
                    causation_id: EventId::hash(2),
                    ts: 3,
                    aggregate_id: AggregateId::new(1),
                },
            ]
        )
    }
}
