// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
use crate::{trap, EType, PanicDispatcher};
use actix::{Actor, Addr, AsyncContext, Handler, Message, Recipient};
use std::{
    cmp::{Ordering, Reverse},
    collections::BinaryHeap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use super::batch_router::FlushSeq;

#[derive(Message)]
#[rtype(result = "()")]
pub struct StartTimelock {
    seq: u64,
    now: u64,
    delay: u64,
}

impl StartTimelock {
    pub fn new(seq: u64, now: u64, delay: u64) -> Self {
        Self { seq, now, delay }
    }
}

pub trait Clock: Send + Sync + 'static {
    fn now_micros(&self) -> u64;
}

// Production implementation
#[derive(Clone, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_micros(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_else(|_| Duration::from_secs(0))
            .as_micros() as u64
    }
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Tick;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct Timelock {
    expiry: u64,
    seq: u64,
}

impl Timelock {
    pub fn new(expiry: u64, seq: u64) -> Self {
        Self { expiry, seq }
    }
}

impl Ord for Timelock {
    fn cmp(&self, other: &Self) -> Ordering {
        self.expiry.cmp(&other.expiry)
    }
}

impl PartialOrd for Timelock {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct TimelockQueue {
    timelocks: BinaryHeap<Reverse<Timelock>>,
    batch_router: Recipient<FlushSeq>,
    clock: Arc<dyn Clock>,
}

impl TimelockQueue {
    pub fn new(batch_router: impl Into<Recipient<FlushSeq>>) -> Self {
        Self::with_clock(batch_router, Arc::new(SystemClock))
    }

    pub fn spawn(batch_router: impl Into<Recipient<FlushSeq>>) -> Addr<Self> {
        Self::new(batch_router).start()
    }

    pub fn with_clock(batch_router: impl Into<Recipient<FlushSeq>>, clock: Arc<dyn Clock>) -> Self {
        Self {
            batch_router: batch_router.into(),
            timelocks: BinaryHeap::new(),
            clock,
        }
    }

    fn next_timelock_lt(&mut self, now: u64) -> bool {
        if let Some(peek) = self.timelocks.peek() {
            peek.0.expiry <= now
        } else {
            false
        }
    }
}

impl Actor for TimelockQueue {
    type Context = actix::Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // Send Tick to self every second
        ctx.run_interval(Duration::from_secs(1), |_, ctx| {
            ctx.address().do_send(Tick);
        });
    }
}

impl Handler<StartTimelock> for TimelockQueue {
    type Result = ();
    fn handle(&mut self, msg: StartTimelock, _: &mut Self::Context) -> Self::Result {
        let expiry = msg.now + msg.delay;
        self.timelocks.push(Reverse(Timelock::new(expiry, msg.seq)));
    }
}

impl Handler<Tick> for TimelockQueue {
    type Result = ();
    fn handle(&mut self, _: Tick, _: &mut Self::Context) -> Self::Result {
        trap(EType::IO, &PanicDispatcher::new(), || {
            let now_time = self.clock.now_micros();

            while self.timelocks.len() > 0 && self.next_timelock_lt(now_time) {
                if let Some(tl) = self.timelocks.pop() {
                    let seq = tl.0.seq;
                    self.batch_router.try_send(FlushSeq(seq))?;
                }
            }
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix::prelude::*;
    use std::sync::{
        atomic::{AtomicU64, Ordering as AtomicOrdering},
        Arc, Mutex,
    };
    use tokio::time::sleep;

    // ==================== Mock Clock ====================

    #[derive(Clone)]
    pub struct MockClock {
        current_time: Arc<AtomicU64>,
    }

    impl MockClock {
        pub fn new(initial_time: u64) -> Self {
            Self {
                current_time: Arc::new(AtomicU64::new(initial_time)),
            }
        }

        pub fn set(&self, time: u64) {
            self.current_time.store(time, AtomicOrdering::SeqCst);
        }

        pub fn advance(&self, micros: u64) {
            self.current_time.fetch_add(micros, AtomicOrdering::SeqCst);
        }
    }

    impl Clock for MockClock {
        fn now_micros(&self) -> u64 {
            self.current_time.load(AtomicOrdering::SeqCst)
        }
    }

    // ==================== Mock Router ====================

    struct MockBatchRouter {
        received_seqs: Arc<Mutex<Vec<u64>>>,
    }

    impl MockBatchRouter {
        fn new(received_seqs: Arc<Mutex<Vec<u64>>>) -> Self {
            Self { received_seqs }
        }
    }

    impl Actor for MockBatchRouter {
        type Context = Context<Self>;
    }

    impl Handler<FlushSeq> for MockBatchRouter {
        type Result = ();

        fn handle(&mut self, msg: FlushSeq, _: &mut Self::Context) -> Self::Result {
            self.received_seqs.lock().unwrap().push(msg.0);
        }
    }

    // ==================== Tests ====================

    #[actix::test]
    async fn test_tick_with_mock_clock_no_expiry() {
        let received_seqs = Arc::new(Mutex::new(Vec::new()));
        let mock_router = MockBatchRouter::new(received_seqs.clone()).start();

        let clock = MockClock::new(1000); // Start at t=1000
        let queue =
            TimelockQueue::with_clock(mock_router.recipient(), Arc::new(clock.clone())).start();

        // Add timelock expiring at t=2000
        queue
            .send(StartTimelock::new(42, 1000, 1000))
            .await
            .unwrap();

        // Tick at t=1000 - nothing should expire
        queue.send(Tick).await.unwrap();
        sleep(Duration::from_millis(10)).await;

        assert!(received_seqs.lock().unwrap().is_empty());
    }

    #[actix::test]
    async fn test_tick_with_mock_clock_after_expiry() {
        let received_seqs = Arc::new(Mutex::new(Vec::new()));
        let mock_router = MockBatchRouter::new(received_seqs.clone()).start();

        let clock = MockClock::new(1000);
        let queue =
            TimelockQueue::with_clock(mock_router.recipient(), Arc::new(clock.clone())).start();

        // Add timelock expiring at t=2000
        queue
            .send(StartTimelock::new(42, 1000, 1000))
            .await
            .unwrap();

        // Advance clock past expiry
        clock.set(2500);

        // Now tick should flush
        queue.send(Tick).await.unwrap();
        sleep(Duration::from_millis(10)).await;

        let seqs = received_seqs.lock().unwrap();
        assert_eq!(seqs.len(), 1);
        assert_eq!(seqs[0], 42);
    }

    #[actix::test]
    async fn test_tick_exact_expiry_boundary() {
        let received_seqs = Arc::new(Mutex::new(Vec::new()));
        let mock_router = MockBatchRouter::new(received_seqs.clone()).start();

        let clock = MockClock::new(1000);
        let queue =
            TimelockQueue::with_clock(mock_router.recipient(), Arc::new(clock.clone())).start();

        // Add timelock expiring at exactly t=2000
        queue
            .send(StartTimelock::new(42, 1000, 1000))
            .await
            .unwrap();

        // Set clock to exact expiry time
        clock.set(2000);

        queue.send(Tick).await.unwrap();
        sleep(Duration::from_millis(10)).await;

        // Should flush at exact expiry (<=)
        let seqs = received_seqs.lock().unwrap();
        assert_eq!(seqs.len(), 1);
        assert_eq!(seqs[0], 42);
    }

    #[actix::test]
    async fn test_tick_one_microsecond_before_expiry() {
        let received_seqs = Arc::new(Mutex::new(Vec::new()));
        let mock_router = MockBatchRouter::new(received_seqs.clone()).start();

        let clock = MockClock::new(1000);
        let queue =
            TimelockQueue::with_clock(mock_router.recipient(), Arc::new(clock.clone())).start();

        // Add timelock expiring at t=2000
        queue
            .send(StartTimelock::new(42, 1000, 1000))
            .await
            .unwrap();

        // Set clock to 1 microsecond before expiry
        clock.set(1999);

        queue.send(Tick).await.unwrap();
        sleep(Duration::from_millis(10)).await;

        // Should NOT flush
        assert!(received_seqs.lock().unwrap().is_empty());
    }

    #[actix::test]
    async fn test_multiple_timelocks_partial_expiry() {
        let received_seqs = Arc::new(Mutex::new(Vec::new()));
        let mock_router = MockBatchRouter::new(received_seqs.clone()).start();

        let clock = MockClock::new(0);
        let queue =
            TimelockQueue::with_clock(mock_router.recipient(), Arc::new(clock.clone())).start();

        // Add timelocks with different expiries
        queue.send(StartTimelock::new(1, 0, 1000)).await.unwrap(); // expires at 1000
        queue.send(StartTimelock::new(2, 0, 2000)).await.unwrap(); // expires at 2000
        queue.send(StartTimelock::new(3, 0, 3000)).await.unwrap(); // expires at 3000

        // Advance to 1500 - only first should expire
        clock.set(1500);
        queue.send(Tick).await.unwrap();
        sleep(Duration::from_millis(10)).await;

        {
            let seqs = received_seqs.lock().unwrap();
            assert_eq!(seqs.len(), 1);
            assert_eq!(seqs[0], 1);
        }

        // Advance to 2500 - second should expire
        clock.set(2500);
        queue.send(Tick).await.unwrap();
        sleep(Duration::from_millis(10)).await;

        {
            let seqs = received_seqs.lock().unwrap();
            assert_eq!(seqs.len(), 2);
            assert_eq!(seqs[1], 2);
        }

        // Advance to 5000 - third should expire
        clock.set(5000);
        queue.send(Tick).await.unwrap();
        sleep(Duration::from_millis(10)).await;

        {
            let seqs = received_seqs.lock().unwrap();
            assert_eq!(seqs.len(), 3);
            assert_eq!(seqs[2], 3);
        }
    }

    #[actix::test]
    async fn test_clock_advance_helper() {
        let received_seqs = Arc::new(Mutex::new(Vec::new()));
        let mock_router = MockBatchRouter::new(received_seqs.clone()).start();

        let clock = MockClock::new(1000);
        let queue =
            TimelockQueue::with_clock(mock_router.recipient(), Arc::new(clock.clone())).start();

        queue.send(StartTimelock::new(42, 1000, 500)).await.unwrap();

        // Use advance instead of set
        clock.advance(600); // Now at 1600, expiry is 1500

        queue.send(Tick).await.unwrap();
        sleep(Duration::from_millis(10)).await;

        assert_eq!(received_seqs.lock().unwrap().len(), 1);
    }
}
