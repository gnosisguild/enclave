use actix::{Message, Recipient};
use alloy::rpc::types::Log;
use e3_events::{CorrelationId, EvmEvent};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HistoricalSyncComplete {
    pub chain_id: u64,
    pub prev_event: Option<CorrelationId>,
    pub id: CorrelationId,
}

impl HistoricalSyncComplete {
    pub fn new(chain_id: u64, prev_event: Option<CorrelationId>) -> Self {
        let id = CorrelationId::new();
        Self {
            id,
            chain_id,
            prev_event,
        }
    }

    pub fn get_id(&self) -> CorrelationId {
        self.id
    }
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub enum EnclaveEvmEvent {
    /// Signal that this reader has completed historical sync
    HistoricalSyncComplete(HistoricalSyncComplete),
    /// An actual event from the blockchain
    Event(EvmEvent),
    /// Raw log data from the provider
    Log(EvmLog),
}

impl EnclaveEvmEvent {
    pub fn get_id(&self) -> CorrelationId {
        match self {
            EnclaveEvmEvent::HistoricalSyncComplete(e) => e.get_id(),
            EnclaveEvmEvent::Log(e) => e.get_id(),
            EnclaveEvmEvent::Event(e) => e.get_id(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EvmLog {
    pub id: CorrelationId,
    pub log: Log,
    pub chain_id: u64,
}

impl EvmLog {
    pub fn new(log: Log, chain_id: u64) -> Self {
        let id = CorrelationId::new();
        Self { log, chain_id, id }
    }

    pub fn get_id(&self) -> CorrelationId {
        self.id
    }
}

#[cfg(test)]
use alloy_primitives::Address;

#[cfg(test)]
impl EvmLog {
    pub fn test_log(address: Address, chain_id: u64) -> EvmLog {
        let id = CorrelationId::new();
        EvmLog {
            log: Log {
                inner: alloy_primitives::Log {
                    address,
                    ..Default::default()
                },
                ..Default::default()
            },
            chain_id,
            id,
        }
    }
}

pub type EvmEventProcessor = Recipient<EnclaveEvmEvent>;
