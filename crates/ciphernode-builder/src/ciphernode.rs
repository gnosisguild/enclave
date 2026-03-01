// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::Addr;
use anyhow::Result;
use e3_data::{DataStore, InMemStore, StoreAddr};
use e3_events::{BusHandle, EnclaveEvent, HistoryCollector};
use e3_net::NetChannelBridge;
use libp2p::PeerId;

/// A Sharable handle to a Ciphernode. NOTE: clones are available for use in the CiphernodeSystem
/// but they cannot await the task.
#[derive(Debug)]
pub struct CiphernodeHandle {
    pub address: String,
    pub store: DataStore,
    pub bus: BusHandle,
    pub history: Option<Addr<HistoryCollector<EnclaveEvent>>>,
    pub errors: Option<Addr<HistoryCollector<EnclaveEvent>>>,
    pub peer_id: PeerId,
    pub channel_bridge: Option<NetChannelBridge>,
}

impl PartialEq for CiphernodeHandle {
    fn eq(&self, other: &Self) -> bool {
        self.address == other.address && self.peer_id == other.peer_id
    }
}

impl Eq for CiphernodeHandle {}

impl CiphernodeHandle {
    pub fn new(
        address: String,
        store: DataStore,
        bus: BusHandle,
        history: Option<Addr<HistoryCollector<EnclaveEvent>>>,
        errors: Option<Addr<HistoryCollector<EnclaveEvent>>>,
        peer_id: PeerId,
        channel_bridge: Option<NetChannelBridge>,
    ) -> Self {
        Self {
            address,
            store,
            bus,
            history,
            errors,
            peer_id,
            channel_bridge,
        }
    }

    pub fn bus(&self) -> &BusHandle {
        &self.bus
    }

    pub fn history(&self) -> Option<Addr<HistoryCollector<EnclaveEvent>>> {
        self.history.clone()
    }

    pub fn errors(&self) -> Option<Addr<HistoryCollector<EnclaveEvent>>> {
        self.errors.clone()
    }

    pub fn address(&self) -> String {
        self.address.clone()
    }

    pub fn store(&self) -> &DataStore {
        &self.store
    }

    pub fn channel_bridge(&self) -> Result<NetChannelBridge> {
        Ok(self.channel_bridge.clone().ok_or(anyhow::anyhow!(
            "No channel bridge exists. We are likely not in test mode"
        ))?)
    }

    pub fn in_mem_store(&self) -> Option<&Addr<InMemStore>> {
        let addr = self.store.get_addr();
        if let StoreAddr::InMem(ref store) = addr {
            return Some(store);
        };

        None
    }
}
