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
use data::Repository;
use std::{env, marker::PhantomData, sync::Arc};
use url::Url;
use zeroize::Zeroizing;
use config::RpcAuth as ConfigRpcAuth;

#[derive(Clone)]
pub enum RPC {
    Http(String),
    Https(String),
    Ws(String),
    Wss(String),
}

impl RPC {
    pub fn from_url(url: &str) -> Result<Self> {
        let parsed = Url::parse(url).context("Invalid URL format")?;
        match parsed.scheme() {
            "http" => Ok(RPC::Http(url.to_string())),
            "https" => Ok(RPC::Https(url.to_string())),
            "ws" => Ok(RPC::Ws(url.to_string())),
            "wss" => Ok(RPC::Wss(url.to_string())),
            _ => bail!("Invalid protocol. Expected: http://, https://, ws://, wss://"),
        }
    }

    pub fn as_http_url(&self) -> String {
        match self {
            RPC::Http(url) | RPC::Https(url) => url.clone(),
            RPC::Ws(url) | RPC::Wss(url) => {
                let mut parsed = Url::parse(url).expect("URL was validated in constructor");
                parsed
                    .set_scheme(if self.is_secure() { "https" } else { "http" })
                    .expect("http(s) are valid schemes");
                parsed.to_string()
            }
        }
    }

    pub fn as_ws_url(&self) -> String {
        match self {
            RPC::Ws(url) | RPC::Wss(url) => url.clone(),
            RPC::Http(url) | RPC::Https(url) => {
                let mut parsed = Url::parse(url).expect("URL was validated in constructor");
                parsed
                    .set_scheme(if self.is_secure() { "wss" } else { "ws" })
                    .expect("ws(s) are valid schemes");
                parsed.to_string()
            }
        }
    }

    pub fn is_websocket(&self) -> bool {
        matches!(self, RPC::Ws(_) | RPC::Wss(_))
    }

    pub fn is_secure(&self) -> bool {
        matches!(self, RPC::Https(_) | RPC::Wss(_))
    }
}

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

#[derive(Clone)]
pub enum RpcAuth {
    None,
    Basic {
        username: String,
        password: String,
    },
    Bearer(String)
}

impl RpcAuth {
    fn to_header_value(&self) -> Option<HeaderValue> {
        match self {
            RpcAuth::None => None,
            RpcAuth::Basic { username, password } => {
                let auth = format!(
                    "Basic {}",
                    STANDARD.encode(format!("{}:{}", username, password))
                );
                HeaderValue::from_str(&auth).ok()
            }
            RpcAuth::Bearer(token) => HeaderValue::from_str(&format!("Bearer {}", token)).ok(),
        }
    }

    fn to_ws_auth(&self) -> Option<Authorization> {
        match self {
            RpcAuth::None => None,
            RpcAuth::Basic { username, password } => {
                Some(Authorization::basic(username, password))
            }
            RpcAuth::Bearer(token) => Some(Authorization::bearer(token)),
        }
    }
}


impl From<ConfigRpcAuth> for RpcAuth {
    fn from(value: ConfigRpcAuth) -> Self {
        match value {
            ConfigRpcAuth::None => RpcAuth::None,
            ConfigRpcAuth::Basic { username, password } => RpcAuth::Basic { username, password },
            ConfigRpcAuth::Bearer(token) => RpcAuth::Bearer(token),
        }
    }
}

impl From<RpcAuth> for ConfigRpcAuth {
    fn from(value: RpcAuth) -> Self {
        match value {
            RpcAuth::None => ConfigRpcAuth::None,
            RpcAuth::Basic { username, password } => ConfigRpcAuth::Basic { username, password },
            RpcAuth::Bearer(token) => ConfigRpcAuth::Bearer(token),
        }
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_rpc_type_conversion() {
        // Test HTTP URLs
        let http = RPC::from_url("http://localhost:8545/").unwrap();
        assert!(matches!(http, RPC::Http(_)));
        assert_eq!(http.as_http_url(), "http://localhost:8545/");
        assert_eq!(http.as_ws_url(), "ws://localhost:8545/");

        // Test HTTPS URLs
        let https = RPC::from_url("https://example.com/").unwrap();
        assert!(matches!(https, RPC::Https(_)));
        assert_eq!(https.as_http_url(), "https://example.com/");
        assert_eq!(https.as_ws_url(), "wss://example.com/");

        // Test WS URLs
        let ws = RPC::from_url("ws://localhost:8545/").unwrap();
        assert!(matches!(ws, RPC::Ws(_)));
        assert_eq!(ws.as_http_url(), "http://localhost:8545/");
        assert_eq!(ws.as_ws_url(), "ws://localhost:8545/");

        // Test WSS URLs
        let wss = RPC::from_url("wss://example.com/").unwrap();
        assert!(matches!(wss, RPC::Wss(_)));
        assert_eq!(wss.as_http_url(), "https://example.com/");
        assert_eq!(wss.as_ws_url(), "wss://example.com/");
    }

    #[test]
    fn test_rpc_type_properties() {
        assert!(!RPC::from_url("http://example.com/").unwrap().is_secure());
        assert!(RPC::from_url("https://example.com/").unwrap().is_secure());
        assert!(!RPC::from_url("ws://example.com/").unwrap().is_secure());
        assert!(RPC::from_url("wss://example.com/").unwrap().is_secure());

        assert!(!RPC::from_url("http://example.com/").unwrap().is_websocket());
        assert!(!RPC::from_url("https://example.com/").unwrap().is_websocket());
        assert!(RPC::from_url("ws://example.com/").unwrap().is_websocket());
        assert!(RPC::from_url("wss://example.com/").unwrap().is_websocket());
    }
}
