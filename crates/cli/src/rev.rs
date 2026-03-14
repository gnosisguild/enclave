// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_console::Console;

pub const GIT_SHA: &str = env!("GIT_SHA");

pub async fn execute(out: Console) -> anyhow::Result<()> {
    e3_console::log!(out, "{}", GIT_SHA);
    Ok(())
}
