// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::*;

/// Purge all ciphernode data
pub async fn execute() -> Result<()> {
    e3_entrypoint::nodes::purge::execute().await?;
    Ok(())
}
