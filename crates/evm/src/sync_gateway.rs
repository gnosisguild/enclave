use crate::events::EnclaveEvmEvent;
use crate::HistoricalSyncComplete;
use actix::{Actor, Handler};
use actix::{Addr, Recipient};
use anyhow::Result;
use anyhow::{bail, Context};
use e3_events::{
    trap, BusHandle, EnclaveEvent, EnclaveEventData, EventId, EventSubscriber, SyncEnd,
    SyncEvmEvent, SyncStart,
};
use e3_events::{EType, EvmEvent};
use e3_events::{Event, EventPublisher};
use tracing::info;

/// The chain gateway
pub struct EvmChainGateway {
    bus: BusHandle,
    status: SyncStatus,
}

#[derive(Clone, Debug)]
enum SyncStatus {
    /// Intial State
    Init,
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
        Self::Init
    }
}

impl SyncStatus {
    pub fn forward_to_sync_actor(&mut self, sender: Recipient<SyncEvmEvent>) -> Result<()> {
        let Self::Init = self else {
            bail!(
                "Cannot change state to ForwardToSyncActor when state is {:?}",
                self
            );
        };

        *self = SyncStatus::ForwardToSyncActor(Some(sender));
        info!("Changed to ForwardToSyncActor");
        Ok(())
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
        info!("Changed to BufferUntilLive");
        Ok(sender)
    }

    pub fn live(&mut self) -> Result<Vec<EvmEvent>> {
        let Self::BufferUntilLive(buffer) = self else {
            bail!("Cannot change state to Live when state is {:?}", self);
        };
        let buffer = std::mem::take(buffer);
        *self = SyncStatus::Live;
        info!("Changed to Live");
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
        self.status.forward_to_sync_actor(sender)?;
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
