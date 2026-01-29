// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{events::EvmEventProcessor, evm_parser::EvmParser};
use actix::{Actor, Addr};
use alloy::{
    primitives::{LogData, B256},
    sol,
    sol_types::SolEvent,
};
use e3_events::EnclaveEventData;
use tracing::{error, trace};

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

impl From<TicketBalanceUpdatedWithChainId> for EnclaveEventData {
    fn from(value: TicketBalanceUpdatedWithChainId) -> Self {
        let payload: e3_events::TicketBalanceUpdated = value.into();
        Self::from(payload)
    }
}

struct ConfigurationUpdatedWithChainId(pub IBondingRegistry::ConfigurationUpdated, pub u64);

impl From<ConfigurationUpdatedWithChainId> for e3_events::ConfigurationUpdated {
    fn from(value: ConfigurationUpdatedWithChainId) -> Self {
        let param_bytes = value.0.parameter.as_slice();
        let param_str = String::from_utf8(
            param_bytes
                .iter()
                .copied()
                .take_while(|&b| b != 0)
                .collect(),
        )
        .unwrap_or_else(|_| value.0.parameter.to_string());

        e3_events::ConfigurationUpdated {
            parameter: param_str,
            old_value: value.0.oldValue,
            new_value: value.0.newValue,
            chain_id: value.1,
        }
    }
}

struct OperatorActivationChangedWithChainId(
    pub IBondingRegistry::OperatorActivationChanged,
    pub u64,
);

impl From<OperatorActivationChangedWithChainId> for e3_events::OperatorActivationChanged {
    fn from(value: OperatorActivationChangedWithChainId) -> Self {
        e3_events::OperatorActivationChanged {
            operator: value.0.operator.to_string(),
            active: value.0.active,
            chain_id: value.1,
        }
    }
}

impl From<OperatorActivationChangedWithChainId> for EnclaveEventData {
    fn from(value: OperatorActivationChangedWithChainId) -> Self {
        let payload: e3_events::OperatorActivationChanged = value.into();
        Self::from(payload)
    }
}

impl From<ConfigurationUpdatedWithChainId> for EnclaveEventData {
    fn from(value: ConfigurationUpdatedWithChainId) -> Self {
        let payload: e3_events::ConfigurationUpdated = value.into();
        Self::from(payload)
    }
}

pub fn extractor(data: &LogData, topic: Option<&B256>, chain_id: u64) -> Option<EnclaveEventData> {
    match topic {
        Some(&IBondingRegistry::TicketBalanceUpdated::SIGNATURE_HASH) => {
            let Ok(event) = IBondingRegistry::TicketBalanceUpdated::decode_log_data(data) else {
                error!("Error parsing event TicketBalanceUpdated after topic was matched!");
                return None;
            };
            Some(EnclaveEventData::from(TicketBalanceUpdatedWithChainId(
                event, chain_id,
            )))
        }
        Some(&IBondingRegistry::OperatorActivationChanged::SIGNATURE_HASH) => {
            let Ok(event) = IBondingRegistry::OperatorActivationChanged::decode_log_data(data)
            else {
                error!("Error parsing event OperatorActivationChanged after topic was matched!");
                return None;
            };
            Some(EnclaveEventData::from(
                OperatorActivationChangedWithChainId(event, chain_id),
            ))
        }
        Some(&IBondingRegistry::ConfigurationUpdated::SIGNATURE_HASH) => {
            let Ok(event) = IBondingRegistry::ConfigurationUpdated::decode_log_data(data) else {
                error!("Error parsing event ConfigurationUpdated after topic was matched!");
                return None;
            };
            Some(EnclaveEventData::from(ConfigurationUpdatedWithChainId(
                event, chain_id,
            )))
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
    pub fn setup(next: &EvmEventProcessor) -> Addr<EvmParser> {
        EvmParser::new(next, extractor).start()
    }
}
