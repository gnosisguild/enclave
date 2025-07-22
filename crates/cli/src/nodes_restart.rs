// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::*;
use e3_entrypoint::nodes::restart;

pub async fn execute(id: &str) -> Result<()> {
    restart::execute(id).await?;
    Ok(())
}
