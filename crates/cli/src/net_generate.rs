// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use e3_config::AppConfig;
use e3_entrypoint::net;

pub async fn execute(config: &AppConfig) -> Result<()> {
    let peer_id = net::keypair::generate::execute(config).await?;
    println!("Generated new keypair with peer ID: {}", peer_id);
    println!("Network keypair has been successfully generated and encrypted.");
    Ok(())
}
