use crate::{
    enclave_sol_reader::EnclaveSolReader,
    enclave_sol_writer::EnclaveSolWriter,
    event_reader::EvmEventReaderState,
    helpers::{ReadonlyProvider, RpcWSClient, SignerProvider, WithChainId},
};
use actix::Addr;
use alloy::transports::BoxTransport;
use anyhow::Result;
use data::Repository;
use events::{EnclaveEvent, EventBus};

pub struct EnclaveSol;
impl EnclaveSol {
    pub async fn attach(
        bus: &Addr<EventBus<EnclaveEvent>>,
        read_provider: &WithChainId<ReadonlyProvider, BoxTransport>,
        write_provider: &WithChainId<SignerProvider<RpcWSClient>, RpcWSClient>,
        contract_address: &str,
        repository: &Repository<EvmEventReaderState>,
        start_block: Option<u64>,
    ) -> Result<()> {
        EnclaveSolReader::attach(
            bus,
            read_provider,
            contract_address,
            repository,
            start_block,
        )
        .await?;
        EnclaveSolWriter::attach(bus, write_provider, contract_address).await?;
        Ok(())
    }
}
