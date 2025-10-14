// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{event_reader::EvmEventReaderState, helpers::EthProvider, EvmEventReader};
use actix::Addr;
use alloy::{
    primitives::{LogData, B256},
    providers::Provider,
    sol,
    sol_types::SolEvent,
};
use anyhow::Result;
use e3_data::Repository;
use e3_events::{EnclaveEvent, EventBus};
use tracing::{error, info, trace};

sol!(
    #[sol(rpc)]
    #[derive(Debug)]
    IBondingRegistry,
    "../../packages/enclave-contracts/artifacts/contracts/interfaces/IBondingRegistry.sol/IBondingRegistry.json"
);

struct TicketBalanceUpdatedWithChainId(pub IBondingRegistry::TicketBalanceUpdated, pub u64);

impl From<TicketBalanceUpdatedWithChainId> for e3_events::TicketBalanceUpdated {
    fn from(value: TicketBalanceUpdatedWithChainId) -> Self {
        e3_events::TicketBalanceUpdated {
            operator: value.0.operator.to_string(),
            delta: value.0.delta,
            new_balance: value.0.newBalance,
            reason: value.0.reason,
            chain_id: value.1,
        }
    }
}

impl From<TicketBalanceUpdatedWithChainId> for EnclaveEvent {
    fn from(value: TicketBalanceUpdatedWithChainId) -> Self {
        let payload: e3_events::TicketBalanceUpdated = value.into();
        EnclaveEvent::from(payload)
    }
}

impl From<IBondingRegistry::OperatorActivationChanged> for e3_events::OperatorActivationChanged {
    fn from(value: IBondingRegistry::OperatorActivationChanged) -> Self {
        e3_events::OperatorActivationChanged {
            operator: value.operator.to_string(),
            active: value.active,
        }
    }
}

impl From<IBondingRegistry::OperatorActivationChanged> for EnclaveEvent {
    fn from(value: IBondingRegistry::OperatorActivationChanged) -> Self {
        let payload: e3_events::OperatorActivationChanged = value.into();
        EnclaveEvent::from(payload)
    }
}

pub fn extractor(data: &LogData, topic: Option<&B256>, chain_id: u64) -> Option<EnclaveEvent> {
    match topic {
        Some(&IBondingRegistry::TicketBalanceUpdated::SIGNATURE_HASH) => {
            let Ok(event) = IBondingRegistry::TicketBalanceUpdated::decode_log_data(data) else {
                error!("Error parsing event TicketBalanceUpdated after topic was matched!");
                return None;
            };
            Some(EnclaveEvent::from(TicketBalanceUpdatedWithChainId(
                event, chain_id,
            )))
        }
        Some(&IBondingRegistry::OperatorActivationChanged::SIGNATURE_HASH) => {
            let Ok(event) = IBondingRegistry::OperatorActivationChanged::decode_log_data(data)
            else {
                error!("Error parsing event OperatorActivationChanged after topic was matched!");
                return None;
            };
            Some(EnclaveEvent::from(event))
        }
        _topic => {
            trace!(
                topic=?_topic,
                "Unknown event was received by BondingRegistry.sol parser but was ignored"
            );
            None
        }
    }
}

/// Connects to BondingRegistry.sol converting EVM events to EnclaveEvents
pub struct BondingRegistrySolReader;

impl BondingRegistrySolReader {
    pub async fn attach<P>(
        bus: &Addr<EventBus<EnclaveEvent>>,
        provider: EthProvider<P>,
        contract_address: &str,
        repository: &Repository<EvmEventReaderState>,
        start_block: Option<u64>,
        rpc_url: String,
    ) -> Result<Addr<EvmEventReader<P>>>
    where
        P: Provider + Clone + 'static,
    {
        let addr = EvmEventReader::attach(
            provider,
            extractor,
            contract_address,
            start_block,
            &bus.clone().into(),
            repository,
            rpc_url,
        )
        .await?;

        info!(address=%contract_address, "BondingRegistrySolReader is listening to address");

        Ok(addr)
    }
}

/// Wrapper for a reader
pub struct BondingRegistrySol;

impl BondingRegistrySol {
    pub async fn attach<P>(
        bus: &Addr<EventBus<EnclaveEvent>>,
        provider: EthProvider<P>,
        contract_address: &str,
        repository: &Repository<EvmEventReaderState>,
        start_block: Option<u64>,
        rpc_url: String,
    ) -> Result<()>
    where
        P: Provider + Clone + 'static,
    {
        BondingRegistrySolReader::attach(
            bus,
            provider,
            contract_address,
            repository,
            start_block,
            rpc_url,
        )
        .await?;
        Ok(())
    }
}
