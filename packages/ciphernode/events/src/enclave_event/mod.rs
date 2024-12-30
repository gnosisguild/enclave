mod ciphernode_added;
mod ciphernode_removed;
mod ciphernode_selected;
mod ciphertext_output_published;
mod decryptionshare_created;
mod die;
mod e3_request_complete;
mod e3_requested;
mod enclave_error;
mod keyshare_created;
mod plaintext_aggregated;
mod publickey_aggregated;
mod shutdown;
mod test_event;

pub use ciphernode_added::*;
pub use ciphernode_removed::*;
pub use ciphernode_selected::*;
pub use ciphertext_output_published::*;
pub use decryptionshare_created::*;
pub use die::*;
pub use e3_request_complete::*;
pub use e3_requested::*;
pub use enclave_error::*;
pub use keyshare_created::*;
pub use plaintext_aggregated::*;
pub use publickey_aggregated::*;
pub use shutdown::*;
use test_event::TestEvent;

use crate::{E3id, EventId};
use actix::Message;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self},
    hash::Hash,
};

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub enum EnclaveEvent {
    KeyshareCreated {
        id: EventId,
        data: KeyshareCreated,
    },
    E3Requested {
        id: EventId,
        data: E3Requested,
    },
    PublicKeyAggregated {
        id: EventId,
        data: PublicKeyAggregated,
    },
    CiphertextOutputPublished {
        id: EventId,
        data: CiphertextOutputPublished,
    },
    DecryptionshareCreated {
        id: EventId,
        data: DecryptionshareCreated,
    },
    PlaintextAggregated {
        id: EventId,
        data: PlaintextAggregated,
    },
    CiphernodeSelected {
        id: EventId,
        data: CiphernodeSelected,
    },
    CiphernodeAdded {
        id: EventId,
        data: CiphernodeAdded,
    },
    CiphernodeRemoved {
        id: EventId,
        data: CiphernodeRemoved,
    },
    EnclaveError {
        id: EventId,
        data: EnclaveError,
    },
    E3RequestComplete {
        id: EventId,
        data: E3RequestComplete,
    },
    Shutdown {
        id: EventId,
        data: Shutdown,
    },
    /// This is a test event to use in testing
    TestEvent {
        id: EventId,
        data: TestEvent,
    },
    // CommitteeSelected,
    // OutputDecrypted,
    // CiphernodeRegistered,
    // CiphernodeDeregistered,
}

impl EnclaveEvent {
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }

    pub fn get_id(&self) -> EventId {
        self.clone().into()
    }

    pub fn is_local_only(&self) -> bool {
        // Add a list of local events
        match self {
            EnclaveEvent::CiphernodeSelected { .. } => true,
            EnclaveEvent::E3Requested { .. } => true,
            EnclaveEvent::CiphernodeAdded { .. } => true,
            EnclaveEvent::CiphernodeRemoved { .. } => true,
            EnclaveEvent::E3RequestComplete { .. } => true,
            EnclaveEvent::Shutdown { .. } => true,
            _ => false,
        }
    }
}

impl From<EnclaveEvent> for EventId {
    fn from(value: EnclaveEvent) -> Self {
        match value {
            EnclaveEvent::KeyshareCreated { id, .. } => id,
            EnclaveEvent::E3Requested { id, .. } => id,
            EnclaveEvent::PublicKeyAggregated { id, .. } => id,
            EnclaveEvent::CiphertextOutputPublished { id, .. } => id,
            EnclaveEvent::DecryptionshareCreated { id, .. } => id,
            EnclaveEvent::PlaintextAggregated { id, .. } => id,
            EnclaveEvent::CiphernodeSelected { id, .. } => id,
            EnclaveEvent::CiphernodeAdded { id, .. } => id,
            EnclaveEvent::CiphernodeRemoved { id, .. } => id,
            EnclaveEvent::EnclaveError { id, .. } => id,
            EnclaveEvent::E3RequestComplete { id, .. } => id,
            EnclaveEvent::Shutdown { id, .. } => id,
            EnclaveEvent::TestEvent { id, .. } => id,
        }
    }
}

impl EnclaveEvent {
    pub fn get_e3_id(&self) -> Option<E3id> {
        match self.clone() {
            EnclaveEvent::KeyshareCreated { data, .. } => Some(data.e3_id),
            EnclaveEvent::E3Requested { data, .. } => Some(data.e3_id),
            EnclaveEvent::PublicKeyAggregated { data, .. } => Some(data.e3_id),
            EnclaveEvent::CiphertextOutputPublished { data, .. } => Some(data.e3_id),
            EnclaveEvent::DecryptionshareCreated { data, .. } => Some(data.e3_id),
            EnclaveEvent::PlaintextAggregated { data, .. } => Some(data.e3_id),
            EnclaveEvent::CiphernodeSelected { data, .. } => Some(data.e3_id),
            _ => None,
        }
    }
    pub fn get_data(&self) -> String {
        match self.clone() {
            EnclaveEvent::KeyshareCreated { data, .. } => format!("{}", data),
            EnclaveEvent::E3Requested { data, .. } => format!("{}", data),
            EnclaveEvent::PublicKeyAggregated { data, .. } => format!("{}", data),
            EnclaveEvent::CiphertextOutputPublished { data, .. } => format!("{}", data),
            EnclaveEvent::DecryptionshareCreated { data, .. } => format!("{}", data),
            EnclaveEvent::PlaintextAggregated { data, .. } => format!("{}", data),
            EnclaveEvent::CiphernodeSelected { data, .. } => format!("{}", data),
            EnclaveEvent::CiphernodeAdded { data, .. } => format!("{}", data),
            EnclaveEvent::CiphernodeRemoved { data, .. } => format!("{}", data),
            EnclaveEvent::E3RequestComplete { data, .. } => format!("{}", data),
            EnclaveEvent::EnclaveError { data, .. } => format!("{:?}", data),
            EnclaveEvent::Shutdown { data, .. } => format!("{:?}", data),
            EnclaveEvent::TestEvent { data, .. } => format!("{:?}", data),
            // _ => "<omitted>".to_string(),
        }
    }
}

impl From<KeyshareCreated> for EnclaveEvent {
    fn from(data: KeyshareCreated) -> Self {
        EnclaveEvent::KeyshareCreated {
            id: EventId::hash(data.clone()),
            data: data.clone(),
        }
    }
}

impl From<E3Requested> for EnclaveEvent {
    fn from(data: E3Requested) -> Self {
        EnclaveEvent::E3Requested {
            id: EventId::hash(data.clone()),
            data: data.clone(),
        }
    }
}

impl From<PublicKeyAggregated> for EnclaveEvent {
    fn from(data: PublicKeyAggregated) -> Self {
        EnclaveEvent::PublicKeyAggregated {
            id: EventId::hash(data.clone()),
            data: data.clone(),
        }
    }
}

impl From<CiphertextOutputPublished> for EnclaveEvent {
    fn from(data: CiphertextOutputPublished) -> Self {
        EnclaveEvent::CiphertextOutputPublished {
            id: EventId::hash(data.clone()),
            data: data.clone(),
        }
    }
}

impl From<DecryptionshareCreated> for EnclaveEvent {
    fn from(data: DecryptionshareCreated) -> Self {
        EnclaveEvent::DecryptionshareCreated {
            id: EventId::hash(data.clone()),
            data: data.clone(),
        }
    }
}

impl From<PlaintextAggregated> for EnclaveEvent {
    fn from(data: PlaintextAggregated) -> Self {
        EnclaveEvent::PlaintextAggregated {
            id: EventId::hash(data.clone()),
            data: data.clone(),
        }
    }
}

impl From<E3RequestComplete> for EnclaveEvent {
    fn from(data: E3RequestComplete) -> Self {
        EnclaveEvent::E3RequestComplete {
            id: EventId::hash(data.clone()),
            data: data.clone(),
        }
    }
}

impl From<CiphernodeSelected> for EnclaveEvent {
    fn from(data: CiphernodeSelected) -> Self {
        EnclaveEvent::CiphernodeSelected {
            id: EventId::hash(data.clone()),
            data: data.clone(),
        }
    }
}

impl From<CiphernodeAdded> for EnclaveEvent {
    fn from(data: CiphernodeAdded) -> Self {
        EnclaveEvent::CiphernodeAdded {
            id: EventId::hash(data.clone()),
            data: data.clone(),
        }
    }
}

impl From<CiphernodeRemoved> for EnclaveEvent {
    fn from(data: CiphernodeRemoved) -> Self {
        EnclaveEvent::CiphernodeRemoved {
            id: EventId::hash(data.clone()),
            data: data.clone(),
        }
    }
}

impl From<EnclaveError> for EnclaveEvent {
    fn from(data: EnclaveError) -> Self {
        EnclaveEvent::EnclaveError {
            id: EventId::hash(data.clone()),
            data: data.clone(),
        }
    }
}

impl From<Shutdown> for EnclaveEvent {
    fn from(data: Shutdown) -> Self {
        EnclaveEvent::Shutdown {
            id: EventId::hash(data.clone()),
            data: data.clone(),
        }
    }
}

impl From<TestEvent> for EnclaveEvent {
    fn from(value: TestEvent) -> Self {
        EnclaveEvent::TestEvent {
            id: EventId::hash(value.clone()),
            data: value.clone(),
        }
    }
}

impl FromError for EnclaveEvent {
    type Error = anyhow::Error;
    fn from_error(err_type: EnclaveErrorType, error: Self::Error) -> Self {
        let error_event = EnclaveError::from_error(err_type, error);
        EnclaveEvent::from(error_event)
    }
}

impl fmt::Display for EnclaveEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("{}({})", self.event_type(), self.get_data()))
    }
}

fn extract_enclave_event_name(s: &str) -> &str {
    let bytes = s.as_bytes();
    for (i, &item) in bytes.iter().enumerate() {
        if item == b' ' || item == b'(' {
            return &s[..i];
        }
    }
    s
}

impl EnclaveEvent {
    pub fn event_type(&self) -> String {
        let s = format!("{:?}", self);
        extract_enclave_event_name(&s).to_string()
    }
}
