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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RpcProtocol {
    Http,
    Https,
    Ws,
    Wss,
}

impl RpcProtocol {
    pub fn is_websocket(&self) -> bool {
        matches!(self, RpcProtocol::Ws | RpcProtocol::Wss)
    }

    pub fn is_secure(&self) -> bool {
        matches!(self, RpcProtocol::Https | RpcProtocol::Wss)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            RpcProtocol::Http => "http",
            RpcProtocol::Https => "https",
            RpcProtocol::Ws => "ws",
            RpcProtocol::Wss => "wss",
        }
    }
}

#[derive(Clone)]
pub struct RPC {
    protocol: RpcProtocol,
    url: Url,
}

impl RPC {
    pub fn from_url(url: &str) -> Result<Self> {
        let parsed = Url::parse(url).context("Invalid URL format")?;
        let protocol = match parsed.scheme() {
            "http" => RpcProtocol::Http,
            "https" => RpcProtocol::Https,
            "ws" => RpcProtocol::Ws,
            "wss" => RpcProtocol::Wss,
            _ => bail!("Invalid protocol. Expected: http://, https://, ws://, wss://"),
        };

        if parsed.host_str().is_none() {
            bail!("URL must contain a host");
        }

        Ok(RPC {
            protocol,
            url: parsed,
        })
    }

    pub fn protocol(&self) -> RpcProtocol {
        self.protocol
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn hostname(&self) -> &str {
        // Safe: validated in from_url() - http(s)/ws(s) schemes always require a host
        self.url.host_str().expect("RPC URL always has a host")
    }

    pub fn port(&self) -> u16 {
        // Safe: http(s)/ws(s) always have known default ports
        self.url
            .port_or_known_default()
            .expect("RPC URL always has a port")
    }

    pub fn host_with_port(&self) -> String {
        format!("{}:{}", self.hostname(), self.port())
    }

    pub fn as_http_url(&self) -> Result<String> {
        if !self.protocol.is_websocket() {
            Ok(self.url.to_string())
        } else {
            let mut parsed = self.url.clone();
            let scheme = if self.protocol.is_secure() {
                "https"
            } else {
                "http"
            };
            parsed
                .set_scheme(scheme)
                .map_err(|_| anyhow!("http(s) are valid schemes"))?;
            Ok(parsed.to_string())
        }
    }

    pub fn as_ws_url(&self) -> Result<String> {
        if self.protocol.is_websocket() {
            Ok(self.url.to_string())
        } else {
            let mut parsed = self.url.clone();
            let scheme = if self.protocol.is_secure() {
                "wss"
            } else {
                "ws"
            };
            parsed
                .set_scheme(scheme)
                .map_err(|_| anyhow!("ws(s) are valid schemes"))?;
            Ok(parsed.to_string())
        }
    }

    pub fn is_websocket(&self) -> bool {
        self.protocol.is_websocket()
    }

    pub fn is_secure(&self) -> bool {
        self.protocol.is_secure()
    }

    pub fn is_local(&self) -> bool {
        match self.hostname() {
            "localhost" | "127.0.0.1" | "::1" => true,
            host => host.starts_with("127."), // 127.0.0.0/8 is all loopback
        }
    }
}

#[derive(Debug, Hash, Eq, Deserialize, Serialize, Clone, PartialEq)]
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
