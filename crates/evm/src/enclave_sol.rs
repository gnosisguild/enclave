// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{
    enclave_sol_reader::EnclaveSolReader, enclave_sol_writer::EnclaveSolWriter,
    events::EvmEventProcessor, evm_parser::EvmParser, helpers::EthProvider,
};
use actix::Addr;
use alloy::providers::{Provider, WalletProvider};
use alloy_primitives::Address;
use anyhow::Result;
use e3_events::BusHandle;

pub struct EnclaveSol;

impl EnclaveSol {
    pub async fn attach<W>(
        processor: &EvmEventProcessor,
        bus: &BusHandle,
        write_provider: EthProvider<W>,
        contract_address: Address,
    ) -> Result<Addr<EvmParser>>
    where
        W: Provider + WalletProvider + Clone + 'static,
    {
        let addr = EnclaveSolReader::setup(processor);

        EnclaveSolWriter::attach(bus, write_provider, contract_address).await?;

        Ok(addr)
    }
}
