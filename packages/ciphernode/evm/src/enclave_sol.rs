use crate::{
    enclave_sol_reader::EnclaveSolReader,
    enclave_sol_writer::EnclaveSolWriter,
    helpers::{ReadonlyProvider, SignerProvider, WithChainId},
    EnclaveSolReaderState,
};
use actix::Addr;
use anyhow::Result;
use data::Repository;
use enclave_core::EventBus;

pub struct EnclaveSol;
impl EnclaveSol {
    pub async fn attach(
        bus: &Addr<EventBus>,
        read_provider: &WithChainId<ReadonlyProvider>,
        write_provider: &WithChainId<SignerProvider>,
        contract_address: &str,
        repository: &Repository<EnclaveSolReaderState>,
    ) -> Result<()> {
        EnclaveSolReader::attach(bus, read_provider, contract_address, repository).await?;
        EnclaveSolWriter::attach(bus, write_provider, contract_address).await?;
        Ok(())
    }
}
