// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
use super::{
    batch_router::{BatchRouter, FlushSeq},
    timelock_queue::{Clock, StartTimelock, SystemClock, TimelockQueue},
    AggregateConfig,
};
use crate::{trap, EType, EnclaveEvent, Insert, InsertBatch, PanicDispatcher};
use actix::{Actor, Addr, Handler, Message, Recipient};
use anyhow::Result;
use std::sync::Arc;

#[derive(Message)]
#[rtype(result = "()")]
struct SetDependencies {
    router: Addr<BatchRouter>,
    timelock: Recipient<StartTimelock>,
}

impl SetDependencies {
    pub fn new(router: Addr<BatchRouter>, timelock: impl Into<Recipient<StartTimelock>>) -> Self {
        Self {
            router: router.into(),
            timelock: timelock.into(),
        }
    }
}

pub struct SnapshotBuffer {
    router: Option<Addr<BatchRouter>>,
    timelock: Option<Recipient<StartTimelock>>,
}

impl SnapshotBuffer {
    pub fn new() -> Self {
        SnapshotBuffer {
            router: None,
            timelock: None,
        }
    }

    pub fn spawn(
        config: &AggregateConfig,
        store: impl Into<Recipient<InsertBatch>>,
    ) -> Result<Addr<Self>> {
        let (addr, _) = Self::with_clock(config, store, Arc::new(SystemClock))?;
        Ok(addr)
    }

    pub fn with_clock(
        config: &AggregateConfig,
        store: impl Into<Recipient<InsertBatch>>,
        clock: Arc<dyn Clock>,
    ) -> Result<(Addr<Self>, Addr<TimelockQueue>)> {
        let addr = Self::new().start();
        let store = store.into();
        let router =
            BatchRouter::with_clock(config, addr.clone(), store.clone(), clock.clone()).start();
        let timelock = TimelockQueue::with_clock(addr.clone(), clock, None).start();
        addr.try_send(SetDependencies::new(router, timelock.clone()))?;
        Ok((addr, timelock))
    }
}

impl Actor for SnapshotBuffer {
    type Context = actix::Context<Self>;
}

impl Handler<FlushSeq> for SnapshotBuffer {
    type Result = ();
    fn handle(&mut self, msg: FlushSeq, _: &mut Self::Context) -> Self::Result {
        trap(EType::IO, &PanicDispatcher::new(), || {
            if let Some(ref router) = self.router {
                router.try_send(msg)?;
            }
            Ok(())
        })
    }
}

impl Handler<StartTimelock> for SnapshotBuffer {
    type Result = ();
    fn handle(&mut self, msg: StartTimelock, _: &mut Self::Context) -> Self::Result {
        trap(EType::IO, &PanicDispatcher::new(), || {
            if let Some(ref timelock) = self.timelock {
                timelock.try_send(msg)?;
            }
            Ok(())
        })
    }
}

impl Handler<SetDependencies> for SnapshotBuffer {
    type Result = ();
    fn handle(&mut self, msg: SetDependencies, _: &mut Self::Context) -> Self::Result {
        let SetDependencies { timelock, router } = msg;
        self.timelock = Some(timelock);
        self.router = Some(router);
    }
}

impl Handler<Insert> for SnapshotBuffer {
    type Result = ();
    fn handle(&mut self, msg: Insert, _: &mut Self::Context) -> Self::Result {
        trap(EType::IO, &PanicDispatcher::new(), || {
            if let Some(ref router) = self.router {
                router.try_send(msg)?;
            }
            Ok(())
        })
    }
}

impl Handler<EnclaveEvent> for SnapshotBuffer {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, _: &mut Self::Context) -> Self::Result {
        trap(EType::IO, &PanicDispatcher::new(), || {
            if let Some(ref router) = self.router {
                router.try_send(msg)?;
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod mock_store {
    use std::mem::replace;

    use crate::InsertBatch;
    use actix::{Actor, Handler, Message};

    #[derive(Message)]
    #[rtype(result = "Vec<InsertBatch>")]
    pub struct GetEvts;

    #[derive(Default)]
    pub struct MockStore {
        evts: Vec<InsertBatch>,
    }

    impl Actor for MockStore {
        type Context = actix::Context<Self>;
    }

    impl Handler<InsertBatch> for MockStore {
        type Result = ();

        fn handle(&mut self, msg: InsertBatch, _: &mut Self::Context) -> Self::Result {
            self.evts.push(msg);
        }
    }

    impl Handler<GetEvts> for MockStore {
        type Result = Vec<InsertBatch>;
        fn handle(&mut self, _: GetEvts, _: &mut Self::Context) -> Self::Result {
            replace(&mut self.evts, Vec::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::timelock_queue::mock_clock::MockClock;
    use super::mock_store::GetEvts;
    use super::{mock_store, SnapshotBuffer};
    use crate::snapshot_buffer::timelock_queue::Tick;
    use crate::{
        AggregateConfig, AggregateId, E3id, EnclaveEvent, EventContext, EventId, Insert,
        InsertBatch, Sequenced, TestEvent,
    };
    use actix::Actor;
    use anyhow::Result;
    use std::collections::HashMap;
    use std::sync::Arc;

    #[actix::test]
    async fn test_snapshot_buffer() -> Result<()> {
        let mut delays = HashMap::new();
        delays.insert(AggregateId::new(0), 30);
        delays.insert(AggregateId::new(1), 60);
        let config = &AggregateConfig::new(delays);
        let store = mock_store::MockStore::default().start();

        let clock = Arc::new(MockClock::new(1000));
        let (buffer, timelock) = SnapshotBuffer::with_clock(config, store.clone(), clock.clone())?;

        let ec = EventContext::new_origin(EventId::hash(1), 1000, AggregateId::new(0), None)
            .sequence(10);

        let enclave_10 =
            EnclaveEvent::<Sequenced>::from_data_ec(TestEvent::new("hello", 10).into(), ec.clone());
        let mut inserts_10 = vec![];
        inserts_10.push(Insert::new_with_context("one", b"one".to_vec(), ec.clone()));
        inserts_10.push(Insert::new_with_context("two", b"two".to_vec(), ec.clone()));

        let ec = EventContext::new_origin(EventId::hash(1), 1000, AggregateId::new(1), None)
            .sequence(11);

        let enclave_11 = EnclaveEvent::<Sequenced>::from_data_ec(
            TestEvent::new("hello", 11)
                .with_e3_id(E3id::new("1", 1)) // Aggregate Id is derived from e3_id on the content
                .into(),
            ec.clone(),
        );
        let mut inserts_11 = vec![];
        inserts_11.push(Insert::new_with_context("one", b"one".to_vec(), ec.clone()));
        inserts_11.push(Insert::new_with_context("two", b"two".to_vec(), ec.clone()));

        let ec = EventContext::new_origin(EventId::hash(1), 1000, AggregateId::new(23), None)
            .sequence(12);
        let enclave_12 = EnclaveEvent::<Sequenced>::from_data_ec(
            TestEvent::new("hello", 10)
                .with_e3_id(E3id::new("2", 23))
                .into(),
            ec.clone(),
        );

        buffer.send(enclave_10).await?;
        buffer.send(inserts_10[0].clone()).await?;
        buffer.send(enclave_11).await?;
        buffer.send(inserts_10[1].clone()).await?;
        buffer.send(inserts_11[0].clone()).await?;
        buffer.send(inserts_11[1].clone()).await?;
        buffer.send(enclave_12).await?;

        // Nothing happens as there has not been enough delay
        clock.set(1020);
        timelock.send(Tick).await?;
        let batches = store.send(GetEvts).await?;
        assert_eq!(0, batches.len());

        // Time is up so lets flush aggregate 0 (but not aggregate 1)
        clock.set(1030);
        timelock.send(Tick).await?;
        let batches = store.send(GetEvts).await?;
        assert_eq!(1, batches.len());
        let InsertBatch(inserts) = batches.first().unwrap();
        assert_eq!(5, inserts.len()); // Have 5 inserts as sequence,block and ts get written

        // Not ready yet
        clock.set(1050);
        timelock.send(Tick).await?;
        let batches = store.send(GetEvts).await?;
        assert_eq!(0, batches.len());

        // Time is up so lets flush aggregate 1
        clock.set(1060);
        timelock.send(Tick).await?;
        let batches = store.send(GetEvts).await?;
        assert_eq!(1, batches.len());
        let InsertBatch(inserts) = batches.first().unwrap();
        assert_eq!(5, inserts.len()); // Have 5 inserts as sequence,block and ts get written

        Ok(())
    }
}
