// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::Result;

/// Purge all local data anc cache
pub async fn execute() -> Result<()> {
    e3_entrypoint::nodes::purge::execute().await?;
    e3_support_scripts::program_cache_purge().await?;
    Ok(())
}
