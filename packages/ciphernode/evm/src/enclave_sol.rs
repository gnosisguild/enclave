use crate::{
    enclave_sol_reader::EnclaveSolReader,
    enclave_sol_writer::EnclaveSolWriter,
    event_reader::EvmEventReaderState,
    helpers::{ReadonlyProvider, SignerProvider, WithChainId},
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
        repository: &Repository<EvmEventReaderState>,
    ) -> Result<()> {
        EnclaveSolReader::attach(bus, read_provider, contract_address, repository).await?;
        EnclaveSolWriter::attach(bus, write_provider, contract_address).await?;
        Ok(())
    }
}
