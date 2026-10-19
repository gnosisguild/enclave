// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::env;

use crate::{
    traits::ProgramSupportApi,
    utils::{ensure_project, run_script, Script, DEV_COMPILE, DEV_START},
};
use anyhow::Result;
use async_trait::async_trait;
use e3_config::ProgramConfig;

/// Runs the project's E3 program unproved, through the program runner in `dev` mode.
#[allow(dead_code)]
pub struct ProgramSupportDev(pub ProgramConfig);

impl ProgramSupportDev {
    async fn run(&self, script: &Script) -> Result<()> {
        let cwd = env::current_dir()?;
        ensure_project(&cwd).await?;
        run_script(&cwd, script, &[]).await
    }
}

#[async_trait]
impl ProgramSupportApi for ProgramSupportDev {
    async fn compile(&self) -> Result<()> {
        self.run(&DEV_COMPILE).await
    }
    async fn start(&self) -> Result<()> {
        self.run(&DEV_START).await
    }
}
