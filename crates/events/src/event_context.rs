// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::{fmt, ops::Deref};

use serde::{Deserialize, Serialize};

use crate::{E3id, EventContext, EventId};

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
pub struct ConcreteEventCtx {
    id: EventId,
    causation_id: EventId,
    origin_id: EventId,
    aggregate_id: AggregateId,
    seq: u64,
    ts: u128,
}

impl ConcreteEventCtx {
    pub fn new(
        id: EventId,
        causation_id: EventId,
        origin_id: EventId,
        aggregate_id: AggregateId,
        seq: u64,
        ts: u128,
    ) -> Self {
        Self {
            id,
            causation_id,
            origin_id,
            aggregate_id,
            seq,
            ts,
        }
    }
}

impl EventContext for ConcreteEventCtx {
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

    fn seq(&self) -> u64 {
        self.seq
    }

    fn ts(&self) -> u128 {
        self.ts
    }
}
