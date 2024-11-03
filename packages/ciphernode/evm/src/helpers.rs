use alloy::{
    network::{Ethereum, EthereumWallet},
    providers::{
        fillers::{
            BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller,
            WalletFiller,
        },
        Identity, Provider, ProviderBuilder, RootProvider,
    },
    signers::local::PrivateKeySigner,
    transports::{BoxTransport, Transport},
};
use anyhow::{bail, Context, Result};
use cipher::Cipher;
use data::Repository;
use std::{env, marker::PhantomData, sync::Arc};
use zeroize::Zeroizing;

/// We need to cache the chainId so we can easily use it in a non-async situation
/// This wrapper just stores the chain_id with the Provider
#[derive(Clone)]
// We have to be generic over T as the transport provider in order to handle different transport
// mechanisms such as the HttpClient etc.
pub struct WithChainId<P, T = BoxTransport>
where
    P: Provider<T>,
    T: Transport + Clone,
{
    provider: Arc<P>,
    chain_id: u64,
    _t: PhantomData<T>,
}

impl<P, T> WithChainId<P, T>
where
    P: Provider<T>,
    T: Transport + Clone,
{
    pub async fn new(provider: P) -> Result<Self> {
        let chain_id = provider.get_chain_id().await?;
        Ok(Self {
            provider: Arc::new(provider),
            chain_id,
            _t: PhantomData,
        })
    }

    pub fn get_provider(&self) -> Arc<P> {
        self.provider.clone()
    }

    pub fn get_chain_id(&self) -> u64 {
        self.chain_id
    }
}

pub type ReadonlyProvider = RootProvider<BoxTransport>;

pub async fn create_readonly_provider(
    rpc_url: &str,
) -> Result<WithChainId<ReadonlyProvider, BoxTransport>> {
    let provider = ProviderBuilder::new()
        .on_builtin(rpc_url)
        .await
        .context("Could not create ReadOnlyProvider")?
        .into();
    Ok(WithChainId::new(provider).await?)
}

pub type SignerProvider = FillProvider<
    JoinFill<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        WalletFiller<EthereumWallet>,
    >,
    RootProvider<BoxTransport>,
    BoxTransport,
    Ethereum,
>;

pub async fn create_provider_with_signer(
    rpc_url: &str,
    signer: &Arc<PrivateKeySigner>,
) -> Result<WithChainId<SignerProvider, BoxTransport>> {
    let wallet = EthereumWallet::from(signer.clone());
    let provider = ProviderBuilder::new()
        .with_recommended_fillers()
        .wallet(wallet)
        .on_builtin(rpc_url)
        .await?;

    Ok(WithChainId::new(provider).await?)
}

pub async fn pull_eth_signer_from_env(var: &str) -> Result<Arc<PrivateKeySigner>> {
    let private_key = env::var(var)?;
    let signer = private_key.parse()?;
    env::remove_var(var);
    Ok(Arc::new(signer))
}

pub async fn get_signer_from_repository(
    repository: Repository<Vec<u8>>,
    cipher: &Arc<Cipher>,
) -> Result<Arc<PrivateKeySigner>> {
    let Some(private_key_encrypted) = repository.read().await? else {
        bail!("No private key was found!")
    };

    let encoded_private_key = Zeroizing::new(cipher.decrypt_data(&private_key_encrypted)?);

    let private_key = Zeroizing::new(String::from_utf8(encoded_private_key.to_vec())?);

    let signer = private_key.parse()?;
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
