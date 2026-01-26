use crate::events::EnclaveEvmEvent;
use crate::HistoricalSyncComplete;
use actix::{Actor, Handler};
use actix::{Addr, Recipient};
use anyhow::Result;
use anyhow::{bail, Context};
use e3_events::{
    trap, BusHandle, EnclaveEvent, EnclaveEventData, EventSubscriber, SyncEnd, SyncEvmEvent,
    SyncStart,
};
use e3_events::{EType, EvmEvent};
use e3_events::{Event, EventPublisher};

/// This component sits between the Evm ingestion for a chain and the Sync actor and the Bus.
/// It coordinates event flow between these components.
pub struct EvmChainGateway {
    bus: BusHandle,
    status: SyncStatus,
}

/// This state machine coordinates the function of the EvmChainGateway
#[derive(Clone, Debug)]
enum SyncStatus {
    /// Intial State
    Init(Vec<EvmEvent>), // Include a buffer to hold events that arrive too early
    /// After SyncStart we forward all events to SyncActor
    ForwardToSyncActor(Option<Recipient<SyncEvmEvent>>),
    /// Once the chain has completed historical sync then we buffer all "live" events until sync is
    /// complete
    BufferUntilLive(Vec<EvmEvent>),
    /// Forward all events directly to the bus
    Live,
}

impl Default for SyncStatus {
    fn default() -> Self {
        Self::Init(Vec::new())
    }
}

impl SyncStatus {
    pub fn forward_to_sync_actor(
        &mut self,
        sender: Recipient<SyncEvmEvent>,
    ) -> Result<Vec<EvmEvent>> {
        let Self::Init(buffer) = self else {
            bail!(
                "Cannot change state to ForwardToSyncActor when state is {:?}",
                self
            );
        };

        let buffer = std::mem::take(buffer);
        *self = SyncStatus::ForwardToSyncActor(Some(sender));
        Ok(buffer)
    }

    pub fn buffer_until_live(&mut self) -> Result<Recipient<SyncEvmEvent>> {
        let Self::ForwardToSyncActor(sender) = self else {
            bail!(
                "Cannot change state to BufferUntilLive when state is {:?}",
                self
            );
        };
        let sender = std::mem::take(sender).context("Cannot call buffer_until_live twice")?;
        *self = SyncStatus::BufferUntilLive(vec![]);
        Ok(sender)
    }

    pub fn live(&mut self) -> Result<Vec<EvmEvent>> {
        let Self::BufferUntilLive(buffer) = self else {
            bail!("Cannot change state to Live when state is {:?}", self);
        };
        let buffer = std::mem::take(buffer);
        *self = SyncStatus::Live;
        Ok(buffer)
    }
}

impl EvmChainGateway {
    pub fn new(bus: &BusHandle) -> Self {
        Self {
            bus: bus.clone(),
            status: SyncStatus::default(),
        }
    }

    pub fn setup(bus: &BusHandle) -> Addr<Self> {
        let addr = Self::new(bus).start();
        bus.subscribe_all(&["SyncStart", "SyncEnd"], addr.clone().recipient());
        addr
    }

    fn handle_sync_start(&mut self, msg: SyncStart) -> Result<()> {
        // Received a SyncStart event from the event bus. Get the sender within that event and forward
        // all events to that actor
        let sender = msg.sender.context("No sender on SyncStart Message")?;
        let mut buffer = self.status.forward_to_sync_actor(sender)?;
        // Drain any events that were buffered early
        for evt in buffer.drain(..) {
            self.handle_receive_evm_event(evt)?;
        }
        Ok(())
    }

    fn handle_sync_end(&mut self, _: SyncEnd) -> Result<()> {
        let buffer = self.status.live()?;
        for evt in buffer {
            self.publish_evm_event(evt)?;
        }
        Ok(())
    }

    fn publish_evm_event(&mut self, msg: EvmEvent) -> Result<()> {
        let (data, ts, _) = msg.split();
        self.bus.publish_from_remote(data, ts)?;
        Ok(())
    }

    fn handle_evm_event(&mut self, msg: EnclaveEvmEvent) -> Result<()> {
        match msg {
            EnclaveEvmEvent::HistoricalSyncComplete(e) => {
                self.handle_historical_sync_complete(e)?;
                Ok(())
            }
            EnclaveEvmEvent::Event(event) => {
                self.handle_receive_evm_event(event)?;
                Ok(())
            }
            _ => panic!("EvmChainGateway is only designed to receive EnclaveEvmEvent::HistoricalSyncComplete or EnclaveEvmEvent::Event events"),
        }
    }

    fn handle_historical_sync_complete(&mut self, event: HistoricalSyncComplete) -> Result<()> {
        let sender = self.status.buffer_until_live()?;
        sender.do_send(SyncEvmEvent::HistoricalSyncComplete(event.chain_id));
        Ok(())
    }

    fn handle_receive_evm_event(&mut self, msg: EvmEvent) -> Result<()> {
        match &mut self.status {
            SyncStatus::BufferUntilLive(buffer) => buffer.push(msg),
            SyncStatus::ForwardToSyncActor(Some(sync_actor)) => {
                sync_actor.do_send(msg.into());
            }
            SyncStatus::Live => self.publish_evm_event(msg)?,
            SyncStatus::Init(buffer) => buffer.push(msg),
            _ => (),
        };
        Ok(())
    }
}

impl Actor for EvmChainGateway {
    type Context = actix::Context<Self>;
}

impl Handler<EnclaveEvent> for EvmChainGateway {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, _: &mut Self::Context) -> Self::Result {
        trap(EType::Evm, &self.bus.clone(), || {
            match msg.into_data() {
                EnclaveEventData::SyncStart(e) => self.handle_sync_start(e)?,
                EnclaveEventData::SyncEnd(e) => self.handle_sync_end(e)?,
                _ => (),
            }
            Ok(())
        })
    }
}

impl Handler<EnclaveEvmEvent> for EvmChainGateway {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvmEvent, ctx: &mut Self::Context) -> Self::Result {
        trap(EType::Evm, &self.bus.clone(), || self.handle_evm_event(msg))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use e3_ciphernode_builder::EventSystem;

    use e3_events::{CorrelationId, TestEvent};
    use std::collections::HashMap;
    use tokio::sync::mpsc;

    struct SyncEventCollector {
        tx: mpsc::UnboundedSender<SyncEvmEvent>,
    }

    impl Actor for SyncEventCollector {
        type Context = actix::Context<Self>;
    }

    impl Handler<SyncEvmEvent> for SyncEventCollector {
        type Result = ();
        fn handle(&mut self, msg: SyncEvmEvent, _: &mut Self::Context) {
            let _ = self.tx.send(msg);
        }
    }

    #[actix::test]
    async fn test_evm_chain_gateway() {
        let system = EventSystem::new("test").with_fresh_bus();
        let bus = system.handle().unwrap();

        let (tx, mut rx) = mpsc::unbounded_channel();
        let collector = SyncEventCollector { tx }.start();

        let addr = EvmChainGateway::setup(&bus);

        let chain_id = 1u64;

        // SyncStart: Init -> ForwardToSyncActor
        bus.publish(SyncStart::new(collector.clone(), HashMap::new()))
            .unwrap();

        // Send EVM event while forwarding - should reach collector
        let evm_event = EvmEvent::new(
            CorrelationId::new(),
            TestEvent {
                msg: "test".to_string(),
                entropy: 1,
            }
            .into(),
            100,
            12345,
            chain_id,
        );
        // This will actually arrive earlier than SyncStart but aught to be buffered
        addr.do_send(EnclaveEvmEvent::Event(evm_event));

        let received = rx.recv().await.unwrap();
        assert!(matches!(received, SyncEvmEvent::Event(_)));

        // HistoricalSyncComplete: ForwardToSyncActor -> BufferUntilLive
        addr.do_send(EnclaveEvmEvent::HistoricalSyncComplete(
            HistoricalSyncComplete::new(chain_id, None),
        ));

        let received = rx.recv().await.unwrap();
        assert!(matches!(received, SyncEvmEvent::HistoricalSyncComplete(_)));

        // Send EVM event while buffering - should be buffered (not received)
        let buffered_event = EvmEvent::new(
            CorrelationId::new(),
            TestEvent {
                msg: "buffered".to_string(),
                entropy: 2,
            }
            .into(),
            101,
            12346,
            chain_id,
        );
        addr.do_send(EnclaveEvmEvent::Event(buffered_event));

        // SyncEnd: BufferUntilLive -> Live (publishes buffered events to bus)
        bus.publish(SyncEnd::new()).unwrap();

        // Allow time for async message processing
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // Verify no more messages were sent to collector (buffered events go to bus, not collector)
        assert!(rx.try_recv().is_err());
    }
}
