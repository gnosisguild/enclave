// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::ops::Deref;

use actix::Message;
use serde::{Deserialize, Serialize};

use crate::{
    event_context::{AggregateId, ConcreteEventCtx},
    EventContext, EventId,
};

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct TypedEvent<T> {
    inner: T,
    ctx: ConcreteEventCtx,
}

impl<T> Deref for TypedEvent<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> EventContext for TypedEvent<T> {
    fn id(&self) -> EventId {
        self.ctx.id()
    }

    fn ts(&self) -> u128 {
        self.ctx.ts()
    }

    fn seq(&self) -> u64 {
        self.ctx.seq()
    }

    fn origin_id(&self) -> EventId {
        self.ctx.origin_id()
    }

    fn causation_id(&self) -> EventId {
        self.ctx.causation_id()
    }

    fn aggregate_id(&self) -> AggregateId {
        self.ctx.aggregate_id()
    }
}

impl<T> From<(T, ConcreteEventCtx)> for TypedEvent<T> {
    fn from(value: (T, ConcreteEventCtx)) -> Self {
        Self {
            inner: value.0,
            ctx: value.1,
        }
    }
}
