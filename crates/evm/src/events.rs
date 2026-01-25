use actix::{Message, Recipient};
use alloy::rpc::types::Log;
use alloy_primitives::Address;
use e3_events::{EventId, EvmEvent};
use serde::{Deserialize, Serialize};

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub enum EnclaveEvmEvent {
    /// Signal that this reader has completed historical sync
    HistoricalSyncComplete(u64),
    /// An actual event from the blockchain
    Event(EvmEvent),
    /// Raw log data from the provider
    Log(EvmLog),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EvmLog {
    pub log: Log,
    pub chain_id: u64,
}

#[cfg(test)]
impl EvmLog {
    pub fn test_log(address: Address, chain_id: u64) -> EvmLog {
        EvmLog {
            log: Log {
                inner: alloy_primitives::Log {
                    address,
                    ..Default::default()
                },
                ..Default::default()
            },
            chain_id,
        }
    }
}

impl EnclaveEvmEvent {
    pub fn get_id(&self) -> EventId {
        EventId::hash(self.clone())
    }
}

pub type EvmEventProcessor = Recipient<EnclaveEvmEvent>;
