// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::*;
use clap::Subcommand;
use e3_config::AppConfig;
use e3_console::Out;

use crate::net_get_peer_id;

#[derive(Subcommand, Debug)]
pub enum NetCommands {
    /// Get the ciphernode's libp2p PeerId
    GetPeerId,
}

pub async fn execute(out: &Out, command: NetCommands, config: &AppConfig) -> Result<()> {
    match command {
        NetCommands::GetPeerId => net_get_peer_id::execute(out, config).await?,
    };

    Ok(())
}
