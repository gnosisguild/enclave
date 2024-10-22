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
use tokio::{select, sync::oneshot};
use tracing::{info, trace};

pub async fn stream_from_evm<P: Provider>(
    provider: WithChainId<P>,
    filter: Filter,
    bus: Recipient<EnclaveEvent>,
    extractor: fn(&LogData, Option<&B256>, u64) -> Option<EnclaveEvent>,
    mut shutdown: oneshot::Receiver<()>,
) {
    match provider
        .get_provider()
        .subscribe_logs(&filter)
        .await
        .context("Could not subscribe to stream")
    {
        Ok(subscription) => {
            let mut stream = subscription.into_stream();
            loop {
                select! {
                    maybe_log = stream.next() => {
                        match maybe_log {
                            Some(log) => {
                                trace!("Received log from EVM");
                                let Some(event) = extractor(log.data(), log.topic0(), provider.get_chain_id())
                                else {
                                    trace!("Failed to extract log from EVM");
                                    continue;
                                };
                                info!("Extracted log from evm sending now.");
                                bus.do_send(event);
                            }
                            None => break, // Stream ended
                        }
                    }
                    _ = &mut shutdown => {
                        info!("Received shutdown signal, stopping EVM stream");
                        break;
                    }
                }
            }
        }
        Err(e) => {
            bus.err(EnclaveErrorType::Evm, e);
        }
    };
    info!("Exiting stream loop");
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
    signer: &Arc<PrivateKeySigner>,
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

pub fn ensure_http_rpc(rpc_url: &str) -> String {
    if rpc_url.starts_with("ws://") {
        return rpc_url.replacen("ws://", "http://", 1);
    } else if rpc_url.starts_with("wss://") {
        return rpc_url.replacen("wss://", "https://", 1);
    }
    rpc_url.to_string()
}

pub fn ensure_ws_rpc(rpc_url: &str) -> String {
    if rpc_url.starts_with("http://") {
        return rpc_url.replacen("http://", "ws://", 1);
    } else if rpc_url.starts_with("https://") {
        return rpc_url.replacen("https://", "wss://", 1);
    }
    rpc_url.to_string()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_ensure_http_rpc() {
        assert_eq!(ensure_http_rpc("http://foo.com"), "http://foo.com");
        assert_eq!(ensure_http_rpc("https://foo.com"), "https://foo.com");
        assert_eq!(ensure_http_rpc("ws://foo.com"), "http://foo.com");
        assert_eq!(ensure_http_rpc("wss://foo.com"), "https://foo.com");
    }
    #[test]
    fn test_ensure_ws_rpc() {
        assert_eq!(ensure_ws_rpc("http://foo.com"), "ws://foo.com");
        assert_eq!(ensure_ws_rpc("https://foo.com"), "wss://foo.com");
        assert_eq!(ensure_ws_rpc("wss://foo.com"), "wss://foo.com");
        assert_eq!(ensure_ws_rpc("ws://foo.com"), "ws://foo.com");
    }
}
