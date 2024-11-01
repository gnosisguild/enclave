use crate::{
    enclave_sol_reader::EnclaveSolReader,
    enclave_sol_writer::EnclaveSolWriter,
    helpers::{ReadonlyProvider, SignerProvider, WithChainId},
};
use actix::Addr;
use anyhow::Result;
use enclave_core::EventBus;

pub struct EnclaveSol;
impl EnclaveSol {
    pub async fn attach(
        bus: &Addr<EventBus>,
        read_provider: &WithChainId<ReadonlyProvider>,
        write_provider: &WithChainId<SignerProvider>,
        contract_address: &str,
    ) -> Result<()> {
        EnclaveSolReader::attach(bus, read_provider, contract_address).await?;
        EnclaveSolWriter::attach(bus, write_provider, contract_address).await?;
        Ok(())
    }
}
