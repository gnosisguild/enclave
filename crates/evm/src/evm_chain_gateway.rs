// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::mem::take;

use crate::events::EnclaveEvmEvent;
use crate::HistoricalSyncComplete;
use actix::{Actor, Handler};
use actix::{Addr, Recipient};
use anyhow::Result;
use anyhow::{bail, Context};
use e3_events::EType;
use e3_events::{
    trap, BusHandle, EnclaveEvent, EnclaveEventData, EventFactory, EventSubscriber, EventType,
    EvmSyncEventsReceived, SyncEnd, SyncStart, Unsequenced,
};
use e3_events::{Event, EventPublisher};
use tracing::info;

/// This component sits between the Evm ingestion for a chain and the Sync actor and the Bus.
/// It coordinates event flow between these components.
pub struct EvmChainGateway {
    bus: BusHandle,
    status: SyncStatus,
}

#[derive(Clone, Default, Debug)]
struct ForwardToSyncActorData {
    pub sender: Option<Recipient<EvmSyncEventsReceived>>,
    pub buffer: Vec<EnclaveEvent<Unsequenced>>,
}

impl ForwardToSyncActorData {
    pub fn add_event(&mut self, event: EnclaveEvent<Unsequenced>) {
        self.buffer.push(event);
    }
}

/// This state machine coordinates the function of the EvmChainGateway
#[derive(Clone, Debug)]
enum SyncStatus {
    /// Intial State
    Init(Vec<EnclaveEvent<Unsequenced>>), // Include a buffer to hold events that arrive too early
    /// After SyncStart we forward all events to SyncActor
    ForwardToSyncActor(ForwardToSyncActorData),
    /// Once the chain has completed historical sync then we buffer all "live" events until sync is
    /// complete
    BufferUntilLive(Vec<EnclaveEvent<Unsequenced>>),
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
        sender: Recipient<EvmSyncEventsReceived>,
    ) -> Result<Vec<EnclaveEvent<Unsequenced>>> {
        let Self::Init(buffer) = self else {
            bail!(
                "Cannot change state to ForwardToSyncActor when state is {:?}",
                self
            );
        };

        let buffer = std::mem::take(buffer);
        *self = SyncStatus::ForwardToSyncActor(ForwardToSyncActorData {
            sender: Some(sender),
            buffer: Vec::new(),
        });
        Ok(buffer)
    }

    pub fn buffer_until_live(&mut self) -> Result<ForwardToSyncActorData> {
        let Self::ForwardToSyncActor(sender) = self else {
            bail!(
                "Cannot change state to BufferUntilLive when state is {:?}",
                self
            );
        };

        let state_data = take(sender);
        *self = SyncStatus::BufferUntilLive(vec![]);
        Ok(state_data)
    }

    pub fn live(&mut self) -> Result<Vec<EnclaveEvent<Unsequenced>>> {
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
        bus.subscribe_all(
            &[EventType::SyncStart, EventType::SyncEnd],
            addr.clone().recipient(),
        );
        addr
    }

    fn handle_sync_start(&mut self, msg: SyncStart) -> Result<()> {
        info!("Processing SyncStart message");
        // Received a SyncStart event from the event bus. Get the sender within that event and forward
        // all events to that actor
        let sender = msg.sender.context("No sender on SyncStart Message")?;
        let mut buffer = self.status.forward_to_sync_actor(sender)?;
        // Drain any events that were buffered early
        for evt in buffer.drain(..) {
            self.process_evm_event(evt)?;
        }
        Ok(())
    }

    fn handle_sync_end(&mut self, _: SyncEnd) -> Result<()> {
        info!("Processing SyncEnd message");
        let buffer = self.status.live()?;
        for evt in buffer {
            self.publish_evm_event(evt)?;
        }
        Ok(())
    }

    fn publish_evm_event(&mut self, msg: EnclaveEvent<Unsequenced>) -> Result<()> {
        self.bus.naked_dispatch(msg);
        Ok(())
    }

    fn handle_evm_event(&mut self, msg: EnclaveEvmEvent) -> Result<()> {
        match msg {
            EnclaveEvmEvent::HistoricalSyncComplete(e) => {
                self.forward_historical_sync_complete(e)?;
                Ok(())
            }
            EnclaveEvmEvent::Event(event) => {
                info!("Received event!");
                let (data,ts,_) = event.split();
                let enclave_event = self.bus.event_from_remote_source(data,None,ts)?;
                self.process_evm_event(enclave_event)?;
                Ok(())
            }
            _ => panic!("EvmChainGateway is only designed to receive EnclaveEvmEvent::HistoricalSyncComplete or EnclaveEvmEvent::Event events"),
        }
    }

    fn forward_historical_sync_complete(&mut self, event: HistoricalSyncComplete) -> Result<()> {
        info!(
            "handling historical sync complete for chain_id({})",
            event.chain_id
        );
        let state = self.status.buffer_until_live()?;
        info!("Sending historical sync complete event to sender.");
        let sender = state
            .sender
            .context("ForwardToSyncActor state must hold a sender")?;
        let event = EvmSyncEventsReceived::new(state.buffer, event.chain_id);
        sender.try_send(event)?;
        Ok(())
    }

    fn process_evm_event(&mut self, msg: EnclaveEvent<Unsequenced>) -> Result<()> {
        match &mut self.status {
            SyncStatus::Init(buffer) => {
                info!("Buffering until Forwarding... {:?}", msg);
                buffer.push(msg);
            }
            SyncStatus::BufferUntilLive(buffer) => {
                info!("Buffering until live... {:?}", msg);
                buffer.push(msg);
            }
            SyncStatus::ForwardToSyncActor(state) => state.add_event(msg),
            SyncStatus::Live => self.publish_evm_event(msg)?,
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
        trap(EType::Evm, &self.bus.with_ec(msg.get_ctx()), || {
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
    fn handle(&mut self, msg: EnclaveEvmEvent, _: &mut Self::Context) -> Self::Result {
        info!("Handler<EnclaveEvmEvent>");
        trap(EType::Evm, &self.bus.clone(), || self.handle_evm_event(msg))
    }
}

#[cfg(test)]
mod tests {
    use crate::EvmEvent;

    use super::*;
    use e3_ciphernode_builder::EventSystem;

    use e3_events::{CorrelationId, EvmEventConfig, EvmEventConfigChain, TakeEvents, TestEvent};
    use tokio::sync::mpsc;
    use tracing_subscriber::{fmt, EnvFilter};

    struct SyncEventCollector {
        tx: mpsc::UnboundedSender<EvmSyncEventsReceived>,
    }

    impl Actor for SyncEventCollector {
        type Context = actix::Context<Self>;
    }

    impl Handler<EvmSyncEventsReceived> for SyncEventCollector {
        type Result = ();
        fn handle(&mut self, msg: EvmSyncEventsReceived, _: &mut Self::Context) {
            let _ = self.tx.send(msg);
        }
    }

    #[actix::test]
    async fn test_evm_chain_gateway() -> Result<()> {
        let _foo = tracing::subscriber::set_default(
            fmt()
                .with_env_filter(EnvFilter::new("info"))
                .with_test_writer()
                .finish(),
        );

        let system = EventSystem::new("test").with_fresh_bus();
        let bus: BusHandle = system.handle()?;

        let history_collector = bus.history();

        let (tx, mut rx) = mpsc::unbounded_channel();
        let collector = SyncEventCollector { tx }.start();

        let addr = EvmChainGateway::setup(&bus);

        let chain_id = 1u64;

        // SyncStart: Init -> ForwardToSyncActor
        let mut evm_config = EvmEventConfig::new();
        evm_config.insert(chain_id, EvmEventConfigChain::new(0));
        bus.publish_without_context(SyncStart::new(collector.clone(), evm_config))
            .unwrap();

        // Send EVM event while forwarding - should reach collector
        let evm_event = EvmEvent::new(
            CorrelationId::new(),
            TestEvent {
                msg: "Before Complete".to_string(),
                entropy: 1,
            }
            .into(),
            100,
            12345,
            chain_id,
        );

        // This will actually arrive earlier than SyncStart but aught to be buffered
        addr.send(EnclaveEvmEvent::Event(evm_event)).await?;

        // HistoricalSyncComplete: ForwardToSyncActor -> BufferUntilLive
        addr.send(EnclaveEvmEvent::HistoricalSyncComplete(
            HistoricalSyncComplete::new(chain_id, None),
        ))
        .await?;

        // Normal Synchronizer will take this and wait for other events before flushing events to
        // the bus here we simulate it
        let received = rx.recv().await.unwrap();
        for event in received.events {
            bus.naked_dispatch(event);
        }

        // Send EVM event while buffering - should be buffered (not received)
        let buffered_event = EvmEvent::new(
            CorrelationId::new(),
            TestEvent {
                msg: "Before SyncEnd".to_string(),
                entropy: 2,
            }
            .into(),
            101,
            12346,
            chain_id,
        );
        addr.send(EnclaveEvmEvent::Event(buffered_event)).await?;

        // The Synchronizer will publish the SyncEnd event when it has all the information it needs
        // and has published everything to the bus
        bus.publish_without_context(SyncEnd::new())?;

        let after_event = EvmEvent::new(
            CorrelationId::new(),
            TestEvent {
                msg: "After SyncEnd".to_string(),
                entropy: 2,
            }
            .into(),
            101,
            12346,
            chain_id,
        );

        addr.send(EnclaveEvmEvent::Event(after_event)).await?;

        let full = history_collector.send(TakeEvents::new(5)).await?;

        let test_events: Vec<String> = full
            .iter()
            .filter_map(|e| {
                if let EnclaveEventData::TestEvent(TestEvent { msg, .. }) = e.get_data() {
                    Some(msg.to_string())
                } else {
                    None
                }
            })
            .collect();

        assert_eq!(
            test_events,
            vec!["Before Complete", "Before SyncEnd", "After SyncEnd"]
        );

        let event_types: Vec<String> = full.iter().map(|e| e.event_type()).collect();

        assert_eq!(
            event_types,
            vec![
                "SyncStart",
                "TestEvent",
                "SyncEnd",
                "TestEvent",
                "TestEvent"
            ]
        );
        Ok(())
    }
}
