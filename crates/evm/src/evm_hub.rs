use actix::{Actor, Addr, Handler};

use crate::events::{EnclaveEvmEvent, EvmEventProcessor};

pub struct EvmHub {
    nexts: Vec<EvmEventProcessor>,
}

impl EvmHub {
    pub fn new(nexts: Vec<EvmEventProcessor>) -> Self {
        Self { nexts }
    }

    pub fn setup(nexts: Vec<EvmEventProcessor>) -> Addr<Self> {
        let addr = Self::new(nexts).start();
        addr
    }
}

impl Actor for EvmHub {
    type Context = actix::Context<Self>;
}

impl Handler<EnclaveEvmEvent> for EvmHub {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvmEvent, ctx: &mut Self::Context) -> Self::Result {
        let EnclaveEvmEvent::Log { .. } = msg.clone() else {
            return;
        };

        for next in self.nexts.clone() {
            next.do_send(msg.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::events::EvmLog;

    use super::*;
    use actix::prelude::*;
    use alloy::primitives::address;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use std::time::Duration;
    use tokio::time::sleep;

    #[actix::test]
    async fn test_evm_hub_forwards_log_events_to_all_processors() {
        // Arrange
        let call_count = Arc::new(AtomicUsize::new(0));

        // Create mock processors that track invocations
        let count1 = call_count.clone();
        let count2 = call_count.clone();

        let processor1 = TestProcessor { call_count: count1 }.start();
        let processor2 = TestProcessor { call_count: count2 }.start();

        let hub = EvmHub::setup(vec![
            processor1.clone().recipient(),
            processor2.clone().recipient(),
        ]);

        let log_event = EnclaveEvmEvent::Log(EvmLog::test_log(
            address!("0x1111111111111111111111111111111111111111"),
            1,
        ));

        hub.send(log_event).await.unwrap();

        sleep(Duration::from_millis(10)).await;
        // Assert
        assert_eq!(call_count.load(Ordering::SeqCst), 2);
    }

    // Helper test actor
    struct TestProcessor {
        call_count: Arc<AtomicUsize>,
    }

    impl Actor for TestProcessor {
        type Context = Context<Self>;
    }

    impl Handler<EnclaveEvmEvent> for TestProcessor {
        type Result = ();

        fn handle(&mut self, _msg: EnclaveEvmEvent, _ctx: &mut Self::Context) {
            self.call_count.fetch_add(1, Ordering::SeqCst);
        }
    }
}
