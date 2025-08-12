// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::path::PathBuf;

use anyhow::Result;

pub async fn execute(
    location: Option<PathBuf>,
    template: Option<String>,
    skip_cleanup: bool,
    verbose: bool,
) -> Result<()> {
    e3_init::execute(location, template, skip_cleanup, verbose).await
}
