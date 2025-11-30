// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{
    enclave_sol_reader::EnclaveSolReader, enclave_sol_writer::EnclaveSolWriter,
    event_reader::EvmEventReaderState, helpers::EthProvider, EnclaveEvmEvent,
};
use actix::{Addr, Recipient};
use alloy::providers::{Provider, WalletProvider};
use anyhow::Result;
use e3_data::Repository;
use e3_events::{BusHandle, EnclaveEvent, EventBus};

pub struct EnclaveSol;

impl EnclaveSol {
    pub async fn attach<R, W>(
        processor: &Recipient<EnclaveEvmEvent>,
        bus: &BusHandle<EnclaveEvent>,
        read_provider: EthProvider<R>,
        write_provider: EthProvider<W>,
        contract_address: &str,
        repository: &Repository<EvmEventReaderState>,
        start_block: Option<u64>,
        rpc_url: String,
    ) -> Result<()>
    where
        R: Provider + Clone + 'static,
        W: Provider + WalletProvider + Clone + 'static,
    {
        EnclaveSolReader::attach(
            processor,
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
