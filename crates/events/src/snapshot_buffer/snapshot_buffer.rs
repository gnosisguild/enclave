use super::{
    batch_router::{BatchRouter, FlushSeq},
    timelock_queue::{StartTimelock, TimelockQueue},
    AggregateConfig,
};
use crate::{trap, EType, Insert, InsertBatch, PanicDispatcher};
use actix::{Actor, Addr, Handler, Message, Recipient};
use anyhow::Result;

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
        let me = Self::new().start();
        let store = store.into();
        let router = BatchRouter::new(config, me.clone(), store.clone()).start();
        let timelock = TimelockQueue::new(me.clone()).start();
        me.try_send(SetDependencies::new(router, timelock))?;
        Ok(me)
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
