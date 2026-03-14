// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;
use e3_config::AppConfig;
use e3_console::Out;

pub async fn execute(out: Out, config: &AppConfig) -> Result<()> {
    let address = e3_entrypoint::wallet::get::execute(config).await?;
    e3_console::log!(out, "{}", address);

    Ok(())
}
