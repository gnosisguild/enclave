// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
use super::{
    batch_router::{BatchRouter, FlushSeq},
    timelock_queue::{Clock, StartTimelock, SystemClock, Tick, TimelockQueue},
    AggregateConfig,
};
use crate::{
    trap, EType, EnclaveEvent, EnclaveEventData, Event, Insert, InsertBatch, PanicDispatcher,
};
use actix::{Actor, Addr, Handler, Message, Recipient};
use anyhow::Result;
use e3_utils::{NotifySync, MAILBOX_LIMIT};
use std::sync::Arc;
use tracing::{debug, info, trace};

#[derive(Message)]
#[rtype(result = "()")]
struct SetDependencies {
    router: Addr<BatchRouter>,
    timelock: Addr<TimelockQueue>,
}

impl SetDependencies {
    pub fn new(router: Addr<BatchRouter>, timelock: Addr<TimelockQueue>) -> Self {
        Self {
            router: router.into(),
            timelock,
        }
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Start;

#[derive(Message)]
#[rtype(result = "()")]
pub struct UpdateDestination(pub Recipient<InsertBatch>);
impl UpdateDestination {
    pub fn new(base: impl Into<Recipient<InsertBatch>>) -> Self {
        Self(base.into())
    }
}
enum SnapshotBufferState {
    Running,
    Paused,
}

pub struct SnapshotBuffer {
    router: Option<Addr<BatchRouter>>,
    timelock: Option<Recipient<StartTimelock>>,
    tickable: Option<Recipient<Tick>>,
    state: SnapshotBufferState,
}

impl SnapshotBuffer {
    pub fn new(started: bool) -> Self {
        SnapshotBuffer {
            router: None,
            timelock: None,
            tickable: None,
            state: if !started {
                SnapshotBufferState::Paused
            } else {
                SnapshotBufferState::Running
            },
        }
    }

    pub fn spawn(
        config: &AggregateConfig,
        store: impl Into<Recipient<InsertBatch>>,
        started: bool,
    ) -> Result<Addr<Self>> {
        info!("spawning SnapshotBuffer...");
        let (addr, _) = Self::with_clock(config, store, Arc::new(SystemClock), Some(1), started)?;
        Ok(addr)
    }

    pub fn with_clock(
        config: &AggregateConfig,
        store: impl Into<Recipient<InsertBatch>>,
        clock: Arc<dyn Clock>,
        interval: Option<u64>,
        started: bool,
    ) -> Result<(Addr<Self>, Addr<TimelockQueue>)> {
        let addr = Self::new(started).start();
        let store = store.into();
        let router =
            BatchRouter::with_clock(config, addr.clone(), store.clone(), clock.clone()).start();
        let timelock = TimelockQueue::with_clock(addr.clone(), clock, interval).start();
        addr.try_send(SetDependencies::new(router, timelock.clone()))?;
        Ok((addr, timelock))
    }
}

impl Actor for SnapshotBuffer {
    type Context = actix::Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.set_mailbox_capacity(MAILBOX_LIMIT);
    }
}

impl Handler<FlushSeq> for SnapshotBuffer {
    type Result = ();
    fn handle(&mut self, msg: FlushSeq, _: &mut Self::Context) -> Self::Result {
        trap(EType::IO, &PanicDispatcher::new(), || {
            let SnapshotBufferState::Running = self.state else {
                return Ok(());
            };
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
            let SnapshotBufferState::Running = self.state else {
                return Ok(());
            };

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
        self.timelock = Some(timelock.clone().into());
        self.tickable = Some(timelock.into());
        self.router = Some(router);
    }
}

impl Handler<Insert> for SnapshotBuffer {
    type Result = ();
    fn handle(&mut self, msg: Insert, _: &mut Self::Context) -> Self::Result {
        trap(EType::IO, &PanicDispatcher::new(), || {
            use SnapshotBufferState as S;
            match (&self.state, msg.ctx(), &self.router) {
                // Doing this to cover when there are context free
                // data store manipulations outside of the sync cycle.
                (S::Paused, None, Some(ref router)) => {
                    debug!(
                        "Paused but received context free Insert. Forwarding to batch router..."
                    );
                    router.try_send(msg)?;
                }
                (S::Running, _, Some(ref router)) => {
                    trace!("Forwarding Insert message to batch router...");
                    router.try_send(msg)?;
                }
                _ => (),
            };
            Ok(())
        })
    }
}

impl Handler<EnclaveEvent> for SnapshotBuffer {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        trap(EType::IO, &PanicDispatcher::new(), || {
            if let EnclaveEventData::SyncEnded(_) = msg.get_data() {
                self.notify_sync(ctx, Start);
            };
            let SnapshotBufferState::Running = self.state else {
                return Ok(());
            };
            if let Some(ref router) = self.router {
                router.try_send(msg)?;
            }
            Ok(())
        })
    }
}

impl Handler<Tick> for SnapshotBuffer {
    type Result = ();
    fn handle(&mut self, msg: Tick, _: &mut Self::Context) -> Self::Result {
        trap(EType::IO, &PanicDispatcher::new(), || {
            if let Some(ref tickable) = self.tickable {
                tickable.try_send(msg)?;
            }
            Ok(())
        })
    }
}

impl Handler<Start> for SnapshotBuffer {
    type Result = ();
    fn handle(&mut self, _: Start, _: &mut Self::Context) -> Self::Result {
        trap(EType::IO, &PanicDispatcher::new(), || {
            info!("SnapshotBuffer is now enabled");
            self.state = SnapshotBufferState::Running;
            Ok(())
        })
    }
}

impl Handler<UpdateDestination> for SnapshotBuffer {
    type Result = ();
    fn handle(&mut self, msg: UpdateDestination, _: &mut Self::Context) -> Self::Result {
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
    use e3_utils::MAILBOX_LIMIT;

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
        AggregateConfig, AggregateId, E3id, EnclaveEvent, EventContext, EventContextAccessors,
        EventContextSeq, EventId, Insert, InsertBatch, Sequenced, SyncEnded, TestEvent,
    };
    use actix::Actor;
    use anyhow::Result;
    use e3_test_helpers::with_tracing;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;
    use tracing::info;

    fn create_ec(ag: usize, seq: u64) -> EventContext<Sequenced> {
        EventContext::new_origin(EventId::hash(1), 1000, AggregateId::new(ag), None).sequence(seq)
    }

    fn create_event(ec: &EventContext<Sequenced>) -> EnclaveEvent {
        EnclaveEvent::<Sequenced>::from_data_ec(
            TestEvent::new("hello", ec.seq())
                .with_e3_id(E3id::new("1", *ec.aggregate_id() as u64))
                .into(),
            ec.clone(),
        )
    }

    #[actix::test]
    async fn test_snapshot_buffer() -> Result<()> {
        let _guard = with_tracing("debug");
        let mut delays = HashMap::new();
        delays.insert(AggregateId::new(0), Duration::from_micros(0));
        delays.insert(AggregateId::new(23), Duration::from_micros(30));
        delays.insert(AggregateId::new(1), Duration::from_micros(60));

        let config = &AggregateConfig::new(delays);
        let store = mock_store::MockStore::default().start();

        let clock = Arc::new(MockClock::new(1000));
        let (buffer, timelock) =
            SnapshotBuffer::with_clock(config, store.clone(), clock.clone(), None, false)?;

        buffer
            .send(EnclaveEvent::from_data_ec(
                SyncEnded::new().into(),
                create_ec(0, 9),
            ))
            .await?;

        info!("TimelockQueue should be empty");
        timelock.send(Tick).await?;

        let ec = create_ec(23, 10);
        let enclave_10 = create_event(&ec);

        let mut inserts_10 = vec![];
        inserts_10.push(Insert::new_with_context("one", b"one".to_vec(), ec.clone()));
        inserts_10.push(Insert::new_with_context("two", b"two".to_vec(), ec.clone()));

        let ec = create_ec(1, 11);
        let enclave_11 = create_event(&ec);

        let mut inserts_11 = vec![];
        inserts_11.push(Insert::new_with_context("one", b"one".to_vec(), ec.clone()));
        inserts_11.push(Insert::new_with_context("two", b"two".to_vec(), ec.clone()));

        let ec = create_ec(0, 12);
        let enclave_12 = create_event(&ec);

        // send event 10
        buffer.send(enclave_10).await?;

        info!("TimelockQueue should hold all seq=9 inserts");
        timelock.send(Tick).await?;

        // send the first insert for seq 10
        buffer.send(inserts_10[0].clone()).await?;

        // send event 11
        info!("Sending event seq=11 this should start the timelock for all the seq=10 inserts");
        buffer.send(enclave_11).await?;

        // send a late insert for 10
        buffer.send(inserts_10[1].clone()).await?;

        // send the other inserts for 11
        buffer.send(inserts_11[0].clone()).await?;
        buffer.send(inserts_11[1].clone()).await?;

        // send event 12
        info!("Sending event seq=12 this should start the timelock for all the seq=11 inserts");
        buffer.send(enclave_12).await?;

        // Nothing happens as there has not been enough delay
        info!("Clock=1020 : Checking for events but there should be nothing that has flushed...");
        clock.set(Duration::from_micros(1020));
        timelock.send(Tick).await?;
        let batches = store.send(GetEvts).await?;
        // assert_eq!(0, batches.len());
        assert_eq!(1, batches.len());
        let InsertBatch(inserts) = batches.first().unwrap();
        assert_eq!(3, inserts.len()); // Have sequence,block and ts written as inserts

        // Time is up so lets flush aggregate 23 (but not aggregate 1)
        info!("Clock=1030 : Checking for events Tick should flush batch 10...");
        clock.set(Duration::from_micros(1030));
        timelock.send(Tick).await?;
        let batches = store.send(GetEvts).await?;
        assert_eq!(1, batches.len());
        let InsertBatch(inserts) = batches.first().unwrap();
        assert_eq!(5, inserts.len()); // Have 5 inserts as sequence,block and ts get written

        // Not ready yet
        info!("Clock=1050 : Not ready yet...");
        clock.set(Duration::from_micros(1050));
        timelock.send(Tick).await?;
        let batches = store.send(GetEvts).await?;
        assert_eq!(0, batches.len());

        // Time is up so lets flush aggregate 1
        info!("Clock=1060 : should have all aggregate 1 changes in batch 11...");
        clock.set(Duration::from_micros(1060));
        timelock.send(Tick).await?;
        let batches = store.send(GetEvts).await?;
        assert_eq!(1, batches.len());
        let InsertBatch(inserts) = batches.first().unwrap();
        assert_eq!(5, inserts.len()); // Have 5 inserts as sequence,block and ts get written

        Ok(())
    }
}
