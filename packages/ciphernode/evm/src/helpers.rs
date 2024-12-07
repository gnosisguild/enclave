use alloy::{
    network::{Ethereum, EthereumWallet},
    providers::{
        fillers::{
            BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller,
            WalletFiller,
        },
        Identity, Provider, ProviderBuilder, RootProvider,
    },
    pubsub::PubSubFrontend,
    rpc::client::RpcClient,
    signers::local::PrivateKeySigner,
    transports::{
        http::{
            reqwest::{
                header::{HeaderMap, HeaderValue, AUTHORIZATION},
                Client,
            },
            Http,
        },
        ws::WsConnect,
        Authorization, BoxTransport, Transport,
    },
};
use anyhow::{bail, Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use cipher::Cipher;
use config::{RpcAuth, RPC};
use data::Repository;
use std::{env, marker::PhantomData, sync::Arc};
use zeroize::Zeroizing;
pub trait AuthConversions {
    fn to_header_value(&self) -> Option<HeaderValue>;
    fn to_ws_auth(&self) -> Option<Authorization>;
}

impl AuthConversions for RpcAuth {
    fn to_header_value(&self) -> Option<HeaderValue> {
        match self {
            RpcAuth::None => None,
            RpcAuth::Basic { username, password } => {
                let auth = format!(
                    "Basic {}",
                    STANDARD.encode(Zeroizing::new(format!("{}:{}", username, password)))
                );
                HeaderValue::from_str(&auth).ok()
            }
            RpcAuth::Bearer(token) => HeaderValue::from_str(&format!("Bearer {}", token)).ok(),
        }
    }

    fn to_ws_auth(&self) -> Option<Authorization> {
        match self {
            RpcAuth::None => None,
            RpcAuth::Basic { username, password } => Some(Authorization::basic(username, password)),
            RpcAuth::Bearer(token) => Some(Authorization::bearer(token)),
        }
    }
}

/// We need to cache the chainId so we can easily use it in a non-async situation
/// This wrapper just stores the chain_id with the Provider
#[derive(Clone)]
// We have to be generic over T as the transport provider in order to handle different transport
// mechanisms such as the HttpClient etc.
pub struct WithChainId<P, T>
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

pub type RpcWSClient = PubSubFrontend;
pub type RpcHttpClient = Http<Client>;
pub type SignerProvider<T> = FillProvider<
    JoinFill<
        JoinFill<
            Identity,
            JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
        >,
        WalletFiller<EthereumWallet>,
    >,
    RootProvider<T>,
    T,
    Ethereum,
>;

pub type ReadonlyProvider = RootProvider<BoxTransport>;

#[derive(Clone)]
pub struct ProviderConfig {
    rpc: RPC,
    auth: RpcAuth,
}

impl ProviderConfig {
    pub fn new(rpc: RPC, auth: RpcAuth) -> Self {
        Self { rpc, auth }
    }

    async fn create_ws_provider(&self) -> Result<RootProvider<BoxTransport>> {
        Ok(ProviderBuilder::new()
            .on_ws(self.create_ws_connect()?)
            .await?
            .boxed())
    }

    async fn create_http_provider(&self) -> Result<RootProvider<BoxTransport>> {
        Ok(ProviderBuilder::new()
            .on_client(self.create_http_client()?)
            .boxed())
    }

    pub async fn create_readonly_provider(
        &self,
    ) -> Result<WithChainId<ReadonlyProvider, BoxTransport>> {
        let provider = if self.rpc.is_websocket() {
            self.create_ws_provider().await?
        } else {
            self.create_http_provider().await?
        };
        WithChainId::new(provider).await
    }

    pub async fn create_ws_signer_provider(
        &self,
        signer: &Arc<PrivateKeySigner>,
    ) -> Result<WithChainId<SignerProvider<RpcWSClient>, RpcWSClient>> {
        let wallet = EthereumWallet::from(signer.clone());
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_ws(self.create_ws_connect()?)
            .await
            .context("Failed to create WS signer provider")?;

        WithChainId::new(provider).await
    }

    pub async fn create_http_signer_provider(
        &self,
        signer: &Arc<PrivateKeySigner>,
    ) -> Result<WithChainId<SignerProvider<RpcHttpClient>, RpcHttpClient>> {
        let wallet = EthereumWallet::from(signer.clone());
        let provider = ProviderBuilder::new()
            .with_recommended_fillers()
            .wallet(wallet)
            .on_client(self.create_http_client()?);
        WithChainId::new(provider).await
    }

    fn create_ws_connect(&self) -> Result<WsConnect> {
        Ok(if let Some(ws_auth) = self.auth.to_ws_auth() {
            WsConnect::new(self.rpc.as_ws_url()?).with_auth(ws_auth)
        } else {
            WsConnect::new(self.rpc.as_ws_url()?)
        })
    }

    fn create_http_client(&self) -> Result<RpcClient<Http<Client>>> {
        let mut headers = HeaderMap::new();
        if let Some(auth_header) = self.auth.to_header_value() {
            headers.insert(AUTHORIZATION, auth_header);
        }
        let client = Client::builder()
            .default_headers(headers)
            .build()
            .context("Failed to create HTTP client")?;
        let http = Http::with_client(client, self.rpc.as_http_url()?.parse()?);
        Ok(RpcClient::new(http, false))
    }
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_rpc_type_conversion() -> Result<()> {
        // Test HTTP URLs
        let http = RPC::from_url("http://localhost:8545/").unwrap();
        assert!(matches!(http, RPC::Http(_)));
        assert_eq!(http.as_http_url()?, "http://localhost:8545/");
        assert_eq!(http.as_ws_url()?, "ws://localhost:8545/");

        // Test HTTPS URLs
        let https = RPC::from_url("https://example.com/").unwrap();
        assert!(matches!(https, RPC::Https(_)));
        assert_eq!(https.as_http_url()?, "https://example.com/");
        assert_eq!(https.as_ws_url()?, "wss://example.com/");

        // Test WS URLs
        let ws = RPC::from_url("ws://localhost:8545/").unwrap();
        assert!(matches!(ws, RPC::Ws(_)));
        assert_eq!(ws.as_http_url()?, "http://localhost:8545/");
        assert_eq!(ws.as_ws_url()?, "ws://localhost:8545/");

        // Test WSS URLs
        let wss = RPC::from_url("wss://example.com/").unwrap();
        assert!(matches!(wss, RPC::Wss(_)));
        assert_eq!(wss.as_http_url()?, "https://example.com/");
        assert_eq!(wss.as_ws_url()?, "wss://example.com/");

        Ok(())
    }

    #[test]
    fn test_rpc_type_properties() {
        assert!(!RPC::from_url("http://example.com/").unwrap().is_secure());
        assert!(RPC::from_url("https://example.com/").unwrap().is_secure());
        assert!(!RPC::from_url("ws://example.com/").unwrap().is_secure());
        assert!(RPC::from_url("wss://example.com/").unwrap().is_secure());

        assert!(!RPC::from_url("http://example.com/").unwrap().is_websocket());
        assert!(!RPC::from_url("https://example.com/")
            .unwrap()
            .is_websocket());
        assert!(RPC::from_url("ws://example.com/").unwrap().is_websocket());
        assert!(RPC::from_url("wss://example.com/").unwrap().is_websocket());
    }
}
