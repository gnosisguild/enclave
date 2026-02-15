// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Message, Recipient};
use alloy::rpc::types::Log;
use anyhow::Result;
use e3_events::{
    BusHandle, CorrelationId, EnclaveEvent, EnclaveEventData, EventFactory, EventSource,
    Unsequenced,
};
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

/// This is a processed EvmEvent specifically typed for the Sync actor
#[derive(Message, Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct EvmEvent {
    data: EnclaveEventData,
    block: u64,
    chain_id: u64,
    ts: u128,
    id: CorrelationId,
}

impl EvmEvent {
    pub fn new(
        id: CorrelationId,
        data: EnclaveEventData,
        block: u64,
        ts: u128,
        chain_id: u64,
    ) -> Self {
        Self {
            id,
            data,
            block,
            ts,
            chain_id,
        }
    }

    pub fn split(self) -> (EnclaveEventData, u128, u64) {
        (self.data, self.ts, self.block)
    }

    pub fn get_id(&self) -> CorrelationId {
        self.id
    }

    pub fn chain_id(&self) -> u64 {
        self.chain_id
    }

    pub fn ts(&self) -> u128 {
        self.ts
    }

    pub fn into_enclave_event(self, bus: &BusHandle) -> Result<EnclaveEvent<Unsequenced>> {
        let data = self.data;
        let ts = self.ts;
        bus.event_from_remote_source(data, None, ts, Some(self.block), EventSource::Evm)
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
    /// Dummy event to report that an event was processed. This is required to ensure that the
    /// appropriate events are ordered correctly
    Processed(CorrelationId),
}

impl EnclaveEvmEvent {
    pub fn get_id(&self) -> CorrelationId {
        match self {
            EnclaveEvmEvent::HistoricalSyncComplete(e) => e.get_id(),
            EnclaveEvmEvent::Log(e) => e.get_id(),
            EnclaveEvmEvent::Event(e) => e.get_id(),
            EnclaveEvmEvent::Processed(id) => id.to_owned(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EvmLog {
    pub id: CorrelationId,
    pub log: Log,
    pub timestamp: u64,
    pub chain_id: u64,
}

impl EvmLog {
    pub fn new(log: Log, chain_id: u64, timestamp: u64) -> Self {
        let id = CorrelationId::new();
        Self {
            log,
            chain_id,
            id,
            timestamp,
        }
    }

    pub fn get_id(&self) -> CorrelationId {
        self.id
    }
}

#[cfg(test)]
use alloy_primitives::Address;

#[cfg(test)]
impl EvmLog {
    pub fn test_log(address: Address, chain_id: u64, timestamp: u64) -> EvmLog {
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
            timestamp,
        }
    }
}

pub type EvmEventProcessor = Recipient<EnclaveEvmEvent>;
