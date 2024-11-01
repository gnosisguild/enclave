use std::marker::PhantomData;

use crate::helpers::{self, WithChainId};
use actix::prelude::*;
use actix::{Addr, Recipient};
use alloy::primitives::{LogData, B256};
use alloy::providers::Provider;
use alloy::transports::{BoxTransport, Transport};
use alloy::{eips::BlockNumberOrTag, primitives::Address, rpc::types::Filter};
use anyhow::{anyhow, Result};
use enclave_core::{BusError, EnclaveErrorType, EnclaveEvent, EventBus, Subscribe};
use tokio::sync::oneshot;
use tracing::info;

pub type ExtractorFn<E> = fn(&LogData, Option<&B256>, u64) -> Option<E>;

/// Connects to Enclave.sol converting EVM events to EnclaveEvents
pub struct EvmEventReader<P, T = BoxTransport>
where
    P: Provider<T> + Clone + 'static,
    T: Transport + Clone + Unpin,
{
    provider: Option<WithChainId<P, T>>,
    contract_address: Address,
    bus: Recipient<EnclaveEvent>,
    extractor: ExtractorFn<EnclaveEvent>,
    shutdown_rx: Option<oneshot::Receiver<()>>,
    shutdown_tx: Option<oneshot::Sender<()>>,
}

impl<P, T> EvmEventReader<P, T>
where
    P: Provider<T> + Clone + 'static,
    T: Transport + Clone + Unpin,
{
    pub fn new(
        bus: &Addr<EventBus>,
        provider: &WithChainId<P, T>,
        extractor: ExtractorFn<EnclaveEvent>,
        contract_address: &Address,
    ) -> Result<Self> {
        let (shutdown_tx, shutdown_rx) = oneshot::channel();
        Ok(Self {
            contract_address: contract_address.clone(),
            provider: Some(provider.clone()),
            extractor,
            bus: bus.clone().into(),
            shutdown_rx: Some(shutdown_rx),
            shutdown_tx: Some(shutdown_tx),
        })
    }

    pub async fn attach(
        bus: &Addr<EventBus>,
        provider: &WithChainId<P, T>,
        extractor: ExtractorFn<EnclaveEvent>,
        contract_address: &str,
    ) -> Result<Addr<Self>> {
        let addr =
            EvmEventReader::new(bus, provider, extractor, &contract_address.parse()?)?.start();

        bus.send(Subscribe::new("Shutdown", addr.clone().into()))
            .await?;

        info!(address=%contract_address, "Evm is listening to address");
        Ok(addr)
    }
}

impl<P, T> Actor for EvmEventReader<P, T>
where
    P: Provider<T> + Clone + 'static,
    T: Transport + Clone + Unpin,
{
    type Context = actix::Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        let bus = self.bus.clone();
        let Some(provider) = self.provider.take() else {
            tracing::error!("Could not start event reader as provider has already been used.");
            return;
        };
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
}

impl<P, T> Handler<EnclaveEvent> for EvmEventReader<P, T>
where
    P: Provider<T> + Clone + 'static,
    T: Transport + Clone + Unpin,
{
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, _: &mut Self::Context) -> Self::Result {
        if let EnclaveEvent::Shutdown { .. } = msg {
            if let Some(shutdown) = self.shutdown_tx.take() {
                let _ = shutdown.send(());
            }
        }
    }
}
