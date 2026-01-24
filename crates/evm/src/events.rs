use actix::{Message, Recipient};
use alloy::rpc::types::Log;
use e3_events::{EnclaveEventData, EventId};
use serde::{Deserialize, Serialize};

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub enum EnclaveEvmEvent {
    /// Register a reader with the coordinator before it starts processing
    RegisterReader,
    /// Signal that this reader has completed historical sync
    HistoricalSyncComplete,
    /// An actual event from the blockchain
    Event(EvmEvent),
    /// Raw log data from the provider
    Log(EvmLog),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EvmEvent {
    pub payload: EnclaveEventData,
    pub block: u64,
    pub ts: u128,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EvmLog {
    pub log: Log,
    pub chain_id: u64,
}

impl EnclaveEvmEvent {
    pub fn get_id(&self) -> EventId {
        EventId::hash(self.clone())
    }
}

pub type EvmEventProcessor = Recipient<EnclaveEvmEvent>;
