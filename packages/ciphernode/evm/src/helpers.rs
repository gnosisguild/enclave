use std::{env, sync::Arc};

use actix::Recipient;
use alloy::{
    network::{Ethereum, EthereumWallet},
    primitives::{LogData, B256},
    providers::{
        fillers::{ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller},
        Identity, Provider, ProviderBuilder, RootProvider,
    },
    rpc::types::Filter,
    signers::local::PrivateKeySigner,
    transports::BoxTransport,
};
use anyhow::{Context, Result};
use enclave_core::{BusError, EnclaveErrorType, EnclaveEvent};
use futures_util::stream::StreamExt;

pub async fn stream_from_evm<P:Provider>(
    provider: WithChainId<P>,
    filter: Filter,
    bus: Recipient<EnclaveEvent>,
    extractor: fn(&LogData, Option<&B256>, u64) -> Option<EnclaveEvent>,
) {
    match provider.get_provider()
        .subscribe_logs(&filter)
        .await
        .context("Could not subscribe to stream")
    {
        Ok(subscription) => {
            let mut stream = subscription.into_stream();
            while let Some(log) = stream.next().await {
                let Some(event) = extractor(log.data(), log.topic0(),provider.get_chain_id()) else {
                    continue;
                };
                bus.do_send(event);
            }
        }
        Err(e) => {
            bus.err(EnclaveErrorType::Evm, e);
        }
    }
}

#[derive(Clone)]
pub struct WithChainId<P>
where
    P: Provider,
{
    provider: Arc<P>,
    chain_id: u64,
}

impl<P> WithChainId<P>
where
    P: Provider,
{
    pub async fn new(provider: P) -> Result<Self> {
        let chain_id = provider.get_chain_id().await?;
        Ok(Self {
            provider: Arc::new(provider),
            chain_id,
        })
    }

    pub fn get_provider(&self) -> Arc<P> {
        self.provider.clone()
    }

    pub fn get_chain_id(&self) -> u64 {
        self.chain_id
    }
}

pub type ReadonlyProvider = WithChainId<RootProvider<BoxTransport>>;

pub async fn create_readonly_provider(rpc_url: &str) -> Result<ReadonlyProvider> {
    let provider = ProviderBuilder::new()
        .on_builtin(rpc_url)
        .await
        .context("Could not create ReadOnlyProvider")?
        .into();
    Ok(ReadonlyProvider::new(provider).await?)
}

pub type SignerProvider = WithChainId<
    FillProvider<
        JoinFill<
            JoinFill<JoinFill<JoinFill<Identity, GasFiller>, NonceFiller>, ChainIdFiller>,
            WalletFiller<EthereumWallet>,
        >,
        RootProvider<BoxTransport>,
        BoxTransport,
        Ethereum,
    >,
>;

pub async fn create_provider_with_signer(
    rpc_url: &str,
    signer: Arc<PrivateKeySigner>,
) -> Result<SignerProvider> {
    let wallet = EthereumWallet::from(signer.clone());
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_builtin(rpc_url)
        .await?;
    Ok(SignerProvider::new(provider).await?)
}

pub async fn pull_eth_signer_from_env(var: &str) -> Result<Arc<PrivateKeySigner>> {
    let private_key = env::var(var)?;
    let signer = private_key.parse()?;
    env::remove_var(var);
    Ok(Arc::new(signer))
}
