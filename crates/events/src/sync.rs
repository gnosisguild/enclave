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
