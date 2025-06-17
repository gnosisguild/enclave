use crate::{
    enclave_sol_reader::EnclaveSolReader, enclave_sol_writer::EnclaveSolWriter,
    event_reader::EvmEventReaderState, helpers::EthProvider,
};
use actix::Addr;
use alloy::providers::Provider;
use anyhow::Result;
use e3_data::Repository;
use e3_events::{EnclaveEvent, EventBus};

pub struct EnclaveSol;

impl EnclaveSol {
    pub async fn attach<R, W>(
        bus: &Addr<EventBus<EnclaveEvent>>,
        read_provider: EthProvider<R>,
        write_provider: EthProvider<W>,
        contract_address: &str,
        repository: &Repository<EvmEventReaderState>,
        start_block: Option<u64>,
        rpc_url: String,
    ) -> Result<()>
    where
        R: Provider + Clone + 'static,
        W: Provider + Clone + 'static,
    {
        EnclaveSolReader::attach(
            bus,
            read_provider,
            contract_address,
            repository,
            start_block,
            rpc_url,
        )
        .await?;

        EnclaveSolWriter::attach(bus, write_provider, contract_address).await?;

        Ok(())
    }
}
