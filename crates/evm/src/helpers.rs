// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use alloy::{
    network::EthereumWallet,
    providers::{
        fillers::{
            BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller,
            SimpleNonceManager, WalletFiller,
        },
        Identity, Provider, ProviderBuilder, RootProvider,
    },
    rpc::types::TransactionReceipt,
    signers::local::PrivateKeySigner,
    transports::{
        http::{
            reqwest::{
                header::{HeaderMap, HeaderValue, AUTHORIZATION},
                Client,
            },
            Http,
        },
        ws::{WebSocketConfig, WsConnect},
        Authorization,
    },
};
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use e3_config::{RpcAuth, RPC};
use e3_crypto::Cipher;
use e3_data::Repository;
use e3_utils::{retry_with_backoff, RetryError};
use std::{env, future::Future, sync::Arc};
use tracing::{info, error};

use e3_utils::evm_error_decoder::extract_and_decode_from_string;

pub trait AuthConversions {
    fn to_header_value(&self) -> Option<HeaderValue>;
    fn to_ws_auth(&self) -> Option<Authorization>;
}

impl AuthConversions for RpcAuth {
    fn to_header_value(&self) -> Option<HeaderValue> {
        match self {
            RpcAuth::None => None,
            RpcAuth::Basic { username, password } => {
                let credentials = STANDARD.encode(format!("{}:{}", username, password));
                HeaderValue::from_str(&format!("Basic {}", credentials)).ok()
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

#[derive(Clone)]
pub struct EthProvider<P> {
    provider: Arc<P>,
    chain_id: u64,
}

impl<P: Provider + Clone> EthProvider<P> {
    pub async fn new(provider: P) -> Result<Self> {
        let chain_id = provider.get_chain_id().await?;
        Ok(Self {
            provider: Arc::new(provider),
            chain_id,
        })
    }

    pub fn provider(&self) -> &P {
        &self.provider
    }

    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }
}

pub struct ProviderConfig {
    rpc: RPC,
    auth: RpcAuth,
}

pub type ConcreteReadProvider = FillProvider<
    JoinFill<
        Identity,
        JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
    >,
    RootProvider,
>;

pub type ConcreteWriteProvider = FillProvider<
    JoinFill<
        JoinFill<
            JoinFill<
                alloy::providers::Identity,
                JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
            >,
            NonceFiller<SimpleNonceManager>,
        >,
        WalletFiller<EthereumWallet>,
    >,
    RootProvider,
>;

impl ProviderConfig {
    pub fn new(rpc: RPC, auth: RpcAuth) -> Self {
        Self { rpc, auth }
    }

    pub async fn create_readonly_provider(&self) -> Result<EthProvider<ConcreteReadProvider>> {
        let provider = if self.rpc.is_websocket() {
            ProviderBuilder::new()
                .connect_ws(self.create_ws_connect()?)
                .await
                .context("Failed to connect to WebSocket RPC. Check if the node is running and URL is correct.")?
        } else {
            ProviderBuilder::new().connect_client(self.create_http_client()?)
        };

        EthProvider::new(provider).await
    }

    pub async fn create_signer_provider(
        &self,
        signer: &PrivateKeySigner,
    ) -> Result<EthProvider<ConcreteWriteProvider>> {
        let wallet = EthereumWallet::from(signer.clone());

        let provider = if self.rpc.is_websocket() {
            ProviderBuilder::new()
                .with_simple_nonce_management()
                .wallet(wallet)
                .connect_ws(self.create_ws_connect()?)
                .await
                .context("Failed to connect to WebSocket RPC. Check if the node is running and URL is correct.")?
        } else {
            ProviderBuilder::new()
                .with_simple_nonce_management()
                .wallet(wallet)
                .connect_client(self.create_http_client()?)
        };

        EthProvider::new(provider).await
    }

    fn create_ws_connect(&self) -> Result<WsConnect> {
        let config = WebSocketConfig::default()
            .max_frame_size(Some(32 * 1024 * 1024))
            .max_message_size(Some(32 * 1024 * 1024));

        let mut ws_connect = WsConnect::new(self.rpc.as_ws_url()?).with_config(config);

        if let Some(auth) = self.auth.to_ws_auth() {
            ws_connect = ws_connect.with_auth(auth);
        }

        Ok(ws_connect)
    }

    fn create_http_client(&self) -> Result<alloy::rpc::client::RpcClient> {
        let mut headers = HeaderMap::new();
        if let Some(auth_header) = self.auth.to_header_value() {
            headers.insert(AUTHORIZATION, auth_header);
        }

        let client = Client::builder()
            .default_headers(headers)
            .build()
            .context("Failed to create HTTP client")?;

        let http = Http::with_client(client, self.rpc.as_http_url()?.parse()?);
        Ok(alloy::rpc::client::RpcClient::new(http, false))
    }
}

pub fn load_signer_from_env(var: &str) -> Result<PrivateKeySigner> {
    let private_key = env::var(var)?;
    env::remove_var(var);
    private_key.parse().map_err(Into::into)
}

pub async fn load_signer_from_repository(
    repository: Repository<Vec<u8>>,
    cipher: &Cipher,
) -> Result<PrivateKeySigner> {
    let encrypted_key = repository
        .read()
        .await?
        .ok_or_else(|| anyhow::anyhow!("No private key found in repository"))?;

    let decrypted = cipher.decrypt_data(&encrypted_key)?;
    let private_key = String::from_utf8(decrypted)?;

    private_key.parse().map_err(Into::into)
}

pub async fn get_current_timestamp() -> Result<u64> {
    let config = e3_config::load_config("_default", None, None)?;
    let chain = config
        .chains()
        .first()
        .ok_or_else(|| anyhow::anyhow!("No chains configured"))?;

    let rpc_url = chain.rpc_url()?;
    let provider = ProviderConfig::new(rpc_url, chain.rpc_auth.clone())
        .create_readonly_provider()
        .await?;

    let block = provider
        .provider()
        .get_block_by_number(alloy::eips::BlockNumberOrTag::Latest)
        .await
        .context("Failed to get latest block")?
        .ok_or_else(|| anyhow::anyhow!("Latest block not found"))?;

    Ok(block.header.timestamp)
}

const TX_RETRY_MAX_ATTEMPTS: u32 = 3;
const TX_RETRY_INITIAL_DELAY_MS: u64 = 2000;

fn should_retry_error(error: &str, retry_on_errors: &[&str]) -> bool {
    if retry_on_errors.is_empty() {
        return true;
    }
    retry_on_errors.iter().any(|code| error.contains(code))
}

pub async fn send_tx_with_retry<F, Fut>(
    operation_name: &str,
    retry_on_errors: &[&str],
    tx_fn: F,
) -> Result<TransactionReceipt>
where
    F: Fn() -> Fut,
    Fut: Future<Output = Result<TransactionReceipt>>,
{
    let op_name = operation_name.to_string();
    let retry_codes: Vec<String> = retry_on_errors.iter().map(|s| s.to_string()).collect();

    retry_with_backoff(
        || {
            let op_name = op_name.clone();
            let retry_codes = retry_codes.clone();
            let fut = tx_fn();
            async move {
                match fut.await {
                    Ok(receipt) => Ok(receipt),
                    Err(e) => {
                        let retry_refs: Vec<&str> =
                            retry_codes.iter().map(|s| s.as_str()).collect();

                        let error_str = format!("{}", e);

                        // try to decode the error to get a clear error name
                        match extract_and_decode_from_string(error_str.as_str()) {
                            Some(decoded) => {
                                // we were able to decode it, now let's check if it's one of the ones
                                // we want to retry 
                                if should_retry_error(&decoded.selector, &retry_refs) {
                                    info!("{}: error, will retry: {}", op_name, decoded.name);
                                    Err(RetryError::Retry(e))
                                } else {
                                    // it's not an error we want to retry but maybe it's still a known error
                                    error!("{}: error, will NOT retry: {}", op_name, decoded.name);
                                    Err(RetryError::Failure(e))
                                }
                            }
                            // if it's not an error we have stored then we should assume 
                            // we are not looking to retry 
                            None => {
                                Err(RetryError::Failure(e))
                            }
                        }
                    }
                }
            }
        },
        TX_RETRY_MAX_ATTEMPTS,
        TX_RETRY_INITIAL_DELAY_MS,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rpc_conversions() -> Result<()> {
        // HTTP/HTTPS
        let http = RPC::from_url("http://localhost:8545/")?;
        assert_eq!(http.as_http_url()?, "http://localhost:8545/");
        assert_eq!(http.as_ws_url()?, "ws://localhost:8545/");
        assert!(!http.is_secure());
        assert!(!http.is_websocket());

        let https = RPC::from_url("https://example.com/")?;
        assert_eq!(https.as_http_url()?, "https://example.com/");
        assert_eq!(https.as_ws_url()?, "wss://example.com/");
        assert!(https.is_secure());
        assert!(!https.is_websocket());

        // WS/WSS
        let ws = RPC::from_url("ws://localhost:8545/")?;
        assert_eq!(ws.as_http_url()?, "http://localhost:8545/");
        assert_eq!(ws.as_ws_url()?, "ws://localhost:8545/");
        assert!(!ws.is_secure());
        assert!(ws.is_websocket());

        let wss = RPC::from_url("wss://example.com/")?;
        assert_eq!(wss.as_http_url()?, "https://example.com/");
        assert_eq!(wss.as_ws_url()?, "wss://example.com/");
        assert!(wss.is_secure());
        assert!(wss.is_websocket());

        Ok(())
    }
}
