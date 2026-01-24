use actix::Recipient;
use actix::{Actor, Handler};
use anyhow::Context;
use anyhow::Result;
use e3_events::{trap, BusHandle, EnclaveEvent, EnclaveEventData, SyncEnd, SyncStart};
use e3_events::{EType, SyncEvmEvent};
use e3_events::{Event, EventPublisher};

use crate::events::EnclaveEvmEvent;

pub struct EvmSyncProcessor {
    bus: BusHandle,
    status: SyncStatus,
}

#[derive(Clone)]
enum SyncStatus {
    Init,
    Syncing(Recipient<SyncEvmEvent>),
    Buffering(Vec<SyncEvmEvent>),
    Live,
}

impl EvmSyncProcessor {
    pub fn new(bus: &BusHandle) -> Self {
        Self {
            bus: bus.clone(),
            status: SyncStatus::Init,
        }
    }

    fn handle_sync_start(&mut self, msg: SyncStart) -> Result<()> {
        let sender = msg.sender.context("No sender on SyncStart Message")?;
        self.status = SyncStatus::Syncing(sender);
        Ok(())
    }

    fn handle_sync_end(&mut self, msg: SyncEnd) {
        self.status = SyncStatus::Live;
    }

    fn handle_evm_event(&mut self, msg: EnclaveEvmEvent) -> Result<()> {
        match msg {
            EnclaveEvmEvent::HistoricalSyncComplete => {
                self.handle_historical_sync_complete()?;
                Ok(())
            }
            EnclaveEvmEvent::Event(event) => {
                self.handle_receive_evm_event(event.payload,event.block)?;
                Ok(())
            }
            _ => panic!("EvmSyncProcessor is only designed to receive EnclaveEvmEvent::HistoricalSyncComplete or EnclaveEvmEvent::Event events"),
        }
    }

    fn handle_historical_sync_complete(&mut self) -> Result<()> {
        self.status = SyncStatus::Buffering(vec![]);
        Ok(())
    }

    fn handle_receive_evm_event(&mut self, event: EnclaveEventData, block: u64) -> Result<()> {
        match &mut self.status {
            SyncStatus::Buffering(buffer) => buffer.push(SyncEvmEvent::new(event, block)),
            SyncStatus::Syncing(sender) => sender.do_send(SyncEvmEvent::new(event, block)),
            SyncStatus::Live => {
                self.bus
                    .publish_from_remote(event, 0 /*convert block or whatever to ts*/)?;
            }
            _ => (),
        };
        Ok(())
    }
}

impl Actor for EvmSyncProcessor {
    type Context = actix::Context<Self>;
}

impl Handler<EnclaveEvent> for EvmSyncProcessor {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, _: &mut Self::Context) -> Self::Result {
        trap(EType::Evm, &self.bus.clone(), || {
            match msg.into_data() {
                EnclaveEventData::SyncStart(e) => self.handle_sync_start(e)?,
                EnclaveEventData::SyncEnd(e) => self.handle_sync_end(e),
                _ => (),
            }
            Ok(())
        })
    }
}

impl Handler<EnclaveEvmEvent> for EvmSyncProcessor {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvmEvent, ctx: &mut Self::Context) -> Self::Result {
        trap(EType::Evm, &self.bus.clone(), || self.handle_evm_event(msg))
    }
}
