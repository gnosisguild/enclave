// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::*;
use e3_config::AppConfig;
use e3_entrypoint::nodes::daemon;

pub async fn execute(
    config: &AppConfig,
    exclude: Vec<String>,
    verbose: u8,
    config_string: Option<String>,
    otel: Option<String>,
    experimental_trbfv: bool,
) -> Result<()> {
    daemon::execute(
        config,
        exclude,
        verbose,
        config_string,
        otel,
        experimental_trbfv,
    )
    .await
}
