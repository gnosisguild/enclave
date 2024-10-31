use std::sync::Arc;

use crate::{
    enclave_sol_reader::EnclaveSolReader,
    enclave_sol_writer::EnclaveSolWriter,
    helpers::{ReadonlyProvider, SignerProvider},
};
use actix::Addr;
use alloy::signers::local::PrivateKeySigner;
use anyhow::Result;
use enclave_core::EventBus;

pub struct EnclaveSol;
impl EnclaveSol {
    pub async fn attach(
        bus: &Addr<EventBus>,
        read_provider: &ReadonlyProvider,
        write_provider: &SignerProvider,
        contract_address: &str,
        signer: &Arc<PrivateKeySigner>,
    ) -> Result<()> {
        EnclaveSolReader::attach(bus, read_provider, contract_address).await?;
        EnclaveSolWriter::attach(bus, write_provider, contract_address, signer).await?;
        Ok(())
    }
}
