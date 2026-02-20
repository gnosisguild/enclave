// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use e3_config::AppConfig;

pub async fn execute(config: &AppConfig) -> Result<()> {
    let address = e3_entrypoint::wallet::get::execute(config).await?;
    println!("{}", address);

    Ok(())
}
