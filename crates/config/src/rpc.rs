// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::anyhow;
use anyhow::bail;
use anyhow::Context;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;
use url::Url;

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

    pub fn as_http_url(&self) -> Result<String> {
        match self {
            RPC::Http(url) | RPC::Https(url) => Ok(url.clone()),
            RPC::Ws(url) | RPC::Wss(url) => {
                let mut parsed =
                    Url::parse(url).context(format!("Failed to parse URL: {}", url))?;
                parsed
                    .set_scheme(if self.is_secure() { "https" } else { "http" })
                    .map_err(|_| anyhow!("http(s) are valid schemes"))?;
                Ok(parsed.to_string())
            }
        }
    }

    pub fn as_ws_url(&self) -> Result<String> {
        match self {
            RPC::Ws(url) | RPC::Wss(url) => Ok(url.clone()),
            RPC::Http(url) | RPC::Https(url) => {
                let mut parsed =
                    Url::parse(url).context(format!("Failed to parse URL: {}", url))?;
                parsed
                    .set_scheme(if self.is_secure() { "wss" } else { "ws" })
                    .map_err(|_| anyhow!("ws(s) are valid schemes"))?;
                Ok(parsed.to_string())
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

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
#[serde(tag = "type", content = "credentials")]
pub enum RpcAuth {
    None,
    Basic { username: String, password: String },
    Bearer(String),
}

impl Default for RpcAuth {
    fn default() -> Self {
        RpcAuth::None
    }
}
