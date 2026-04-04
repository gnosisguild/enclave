// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use clap::Subcommand;
use e3_config::AppConfig;
use e3_console::{log, Console};

#[derive(Subcommand, Clone, Debug)]
pub enum ConfigCommands {
    /// Get a config parameter
    Get {
        /// The config parameter to get. If not provided, prints all config values
        param: Option<String>,
    },
}

pub async fn execute(out: Console, command: ConfigCommands, config: &AppConfig) -> Result<()> {
    let ConfigCommands::Get { param } = command;
    match param.as_deref() {
        Some("name") => {
            log!(out, "{}", config.name());
        }
        Some("peers") => {
            for peer in config.peers() {
                log!(out, "{}", peer);
            }
        }
        Some("quic_port") => {
            log!(out, "{}", config.quic_port());
        }
        Some("ctrl_port") => {
            log!(out, "{}", config.ctrl_port());
        }
        Some("address") => {
            if let Some(addr) = config.address() {
                log!(out, "{}", addr);
            }
        }
        Some("role") => {
            log!(out, "{:?}", config.role());
        }
        Some("autonetkey") => {
            log!(out, "{}", config.autonetkey());
        }
        Some("autopassword") => {
            log!(out, "{}", config.autopassword());
        }
        Some("autowallet") => {
            log!(out, "{}", config.autowallet());
        }
        Some("otel") => {
            if let Some(otel) = config.otel() {
                log!(out, "{}", otel);
            }
        }
        Some("config_file") => {
            log!(out, "{}", config.config_file().display());
        }
        Some("config_yaml") => {
            log!(out, "{}", config.config_yaml().display());
        }
        Some("db_file") => {
            log!(out, "{}", config.db_file().display());
        }
        Some("key_file") => {
            log!(out, "{}", config.key_file().display());
        }
        Some("log_file") => {
            log!(out, "{}", config.log_file().display());
        }
        Some("work_dir") => {
            log!(out, "{}", config.work_dir().display());
        }
        Some("chains") => {
            for chain in config.chains() {
                log!(out, "{}", chain.name);
            }
        }
        Some("nodes") => {
            for (name, node_def) in config.nodes() {
                log!(out, "{}: {:?}", name, node_def);
            }
        }
        Some("program") => {
            log!(out, "{:?}", config.program());
        }
        Some(param) => {
            anyhow::bail!("Unknown config parameter: {}", param);
        }
        None => {
            log!(out, "name: {}", config.name());
            log!(out, "peers: {:?}", config.peers());
            log!(out, "quic_port: {}", config.quic_port());
            log!(out, "ctrl_port: {}", config.ctrl_port());
            log!(out, "address: {:?}", config.address());
            log!(out, "role: {:?}", config.role());
            log!(out, "autonetkey: {}", config.autonetkey());
            log!(out, "autopassword: {}", config.autopassword());
            log!(out, "autowallet: {}", config.autowallet());
            log!(out, "otel: {:?}", config.otel());
            log!(out, "config_file: {}", config.config_file().display());
            log!(out, "config_yaml: {}", config.config_yaml().display());
            log!(out, "db_file: {}", config.db_file().display());
            log!(out, "key_file: {}", config.key_file().display());
            log!(out, "log_file: {}", config.log_file().display());
            log!(out, "work_dir: {}", config.work_dir().display());
            log!(out, "chains: {:?}", config.chains());
            log!(out, "nodes: {:?}", config.nodes());
            log!(out, "program: {:?}", config.program());
        }
    }
    Ok(())
}
