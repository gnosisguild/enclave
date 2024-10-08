use std::sync::Arc;

use crate::{enclave_sol_reader::EnclaveSolReader, enclave_sol_writer::EnclaveSolWriter};
use actix::Addr;
use alloy::signers::local::PrivateKeySigner;
use anyhow::Result;
use enclave_core::EventBus;

pub struct EnclaveSol;
impl EnclaveSol {
    pub async fn attach(
        bus: Addr<EventBus>,
        rpc_url: &str,
        contract_address: &str,
        signer: Arc<PrivateKeySigner>,
    ) -> Result<()> {
        EnclaveSolReader::attach(bus.clone(), rpc_url, contract_address).await?;
        EnclaveSolWriter::attach(bus, rpc_url, contract_address, signer).await?;
        Ok(())
    }
}
