use anyhow::Result;
use async_trait::async_trait;
use e3_config::ProgramConfig;

use crate::{
    program_dev::ProgramSupportDev, program_risc0::ProgramSupportRisc0, traits::ProgramSupportApi,
};

fn get_mode(config: ProgramConfig, mode: Option<bool>) -> bool {
    println!("*************************************************************");
    if let Some(m) = mode {
        println!("USING PASSED IN MODE! WHICH IS {}", m);
        return m;
    };
    println!("USING CONFIG MODE! WHICH IS {}", config.dev());

    config.dev()
}

pub enum ProgramSupport {
    Dev(ProgramSupportDev),
    Risc0(ProgramSupportRisc0),
}

impl ProgramSupport {
    pub fn new(config: ProgramConfig, mode: Option<bool>) -> ProgramSupport {
        if get_mode(config.clone(), mode) {
            ProgramSupport::Dev(ProgramSupportDev(config))
        } else {
            ProgramSupport::Risc0(ProgramSupportRisc0(config))
        }
    }
}

#[async_trait]
impl ProgramSupportApi for ProgramSupport {
    async fn compile(&self) -> Result<()> {
        match self {
            ProgramSupport::Dev(s) => s.compile().await,
            ProgramSupport::Risc0(s) => s.compile().await,
        }
    }
    async fn start(&self) -> Result<()> {
        match self {
            ProgramSupport::Dev(s) => s.start().await,
            ProgramSupport::Risc0(s) => s.start().await,
        }
    }
}
