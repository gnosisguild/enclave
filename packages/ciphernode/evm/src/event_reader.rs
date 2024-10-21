use crate::helpers::{self, create_readonly_provider, ReadonlyProvider};
use actix::prelude::*;
use actix::{Addr, Recipient};
use alloy::primitives::{LogData, B256};
use alloy::{
    eips::BlockNumberOrTag, primitives::Address, rpc::types::Filter, sol, sol_types::SolEvent,
};
use anyhow::{anyhow, Result};
use enclave_core::{BusError, EnclaveErrorType, EnclaveEvent, EventBus, Shutdown, Subscribe};
use tokio::sync::oneshot;
use tracing::info;

pub type ExtractorFn = fn(&LogData, Option<&B256>, u64) -> Option<EnclaveEvent>;

/// Connects to Enclave.sol converting EVM events to EnclaveEvents
pub struct EvmEventReader {
    provider: ReadonlyProvider,
    contract_address: Address,
    bus: Recipient<EnclaveEvent>,
    extractor: fn(&LogData, Option<&B256>, u64) -> Option<EnclaveEvent>,
    shutdown_rx: Option<oneshot::Receiver<()>>,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl EvmEventReader {
    pub async fn new(
        bus: Addr<EventBus>,
        rpc_url: &str,
        extractor: ExtractorFn,
        contract_address: Address,
    ) -> Result<Self> {
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        let provider = create_readonly_provider(rpc_url).await?;
        Ok(Self {
            contract_address,
            provider,
            extractor,
            bus: bus.into(),
            shutdown_rx: Some(shutdown_rx),
            shutdown_tx: Some(shutdown_tx),
        })
    }

    pub async fn attach(
        bus: Addr<EventBus>,
        rpc_url: &str,
        extractor: ExtractorFn,
        contract_address: &str,
    ) -> Result<Addr<Self>> {
        let addr = EvmEventReader::new(bus.clone(), rpc_url, extractor, contract_address.parse()?)
            .await?
            .start();

        bus.send(Subscribe::new("Shutdown", addr.clone().into()))
            .await?;

        info!(address=%contract_address, "Evm is listening to address");
        Ok(addr)
    }
}

impl Actor for EvmEventReader {
    type Context = actix::Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        let bus = self.bus.clone();
        let provider = self.provider.clone();
        let filter = Filter::new()
            .address(self.contract_address)
            .from_block(BlockNumberOrTag::Latest);
        let extractor = self.extractor;
        let Some(shutdown) = self.shutdown_rx.take() else {
            bus.err(EnclaveErrorType::Evm, anyhow!("shutdown already called"));
            return;
        };

        ctx.spawn(
            async move { helpers::stream_from_evm(provider, filter, bus, extractor, shutdown).await }
                .into_actor(self),
        );
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        if let Some(shutdown) = self.shutdown_tx.take() {
            let _ = shutdown.send(());
        }

        Running::Stop
    }
}

impl Handler<EnclaveEvent> for EvmEventReader {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, _: &mut Self::Context) -> Self::Result {
        if let EnclaveEvent::Shutdown { .. } = msg {
            if let Some(shutdown) = self.shutdown_tx.take() {
                let _ = shutdown.send(());
            }
        }
    }
}
