// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::EvmEvent;
use actix::Message;
use serde::{Deserialize, Serialize};

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub enum SyncEvmEvent {
    /// Signal that this reader has completed historical sync
    HistoricalSyncComplete(u64),
    /// An actual event from the blockchain
    Event(EvmEvent),
}

impl From<EvmEvent> for SyncEvmEvent {
    fn from(event: EvmEvent) -> SyncEvmEvent {
        SyncEvmEvent::Event(event)
    }
}
