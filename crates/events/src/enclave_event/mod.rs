// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

mod ciphernode_added;
mod ciphernode_removed;
mod ciphernode_selected;
mod ciphertext_output_published;
mod committee_published;
mod compute_request;
mod decryptionshare_created;
mod die;
mod e3_request_complete;
mod e3_requested;
mod enclave_error;
mod keyshare_created;
mod plaintext_aggregated;
mod plaintext_output_published;
mod publickey_aggregated;
mod publish_document;
mod shutdown;
mod test_event;
mod ticket_balance_updated;

pub use ciphernode_added::*;
pub use ciphernode_removed::*;
pub use ciphernode_selected::*;
pub use ciphertext_output_published::*;
pub use committee_published::*;
pub use compute_request::*;
pub use decryptionshare_created::*;
pub use die::*;
pub use e3_request_complete::*;
pub use e3_requested::*;
pub use enclave_error::*;
pub use keyshare_created::*;
pub use plaintext_aggregated::*;
pub use plaintext_output_published::*;
pub use publickey_aggregated::*;
pub use publish_document::*;
pub use shutdown::*;
pub use test_event::*;
pub use ticket_balance_updated::*;

use crate::{E3id, ErrorEvent, Event, EventId};
use actix::Message;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{self},
    hash::Hash,
};

/// Macro to help define From traits for EnclaveEvent
macro_rules! impl_from_event {
    ($($variant:ident),*) => {
        $(
            impl From<$variant> for EnclaveEvent {
                fn from(data: $variant) -> Self {
                    EnclaveEvent::$variant {
                        id: EventId::hash(data.clone()),
                        data: data.clone(),
                    }
                }
            }
        )*
    };
}

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
    PublishDocumentRequested {
        id: EventId,
        data: PublishDocumentRequested,
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
    TicketBalanceUpdated {
        id: EventId,
        data: TicketBalanceUpdated,
    },
    CommitteePublished {
        id: EventId,
        data: CommitteePublished,
    },
    PlaintextOutputPublished {
        id: EventId,
        data: PlaintextOutputPublished,
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
}

impl Event for EnclaveEvent {
    type Id = EventId;

    fn event_type(&self) -> String {
        let s = format!("{:?}", self);
        extract_enclave_event_name(&s).to_string()
    }

    fn event_id(&self) -> Self::Id {
        self.get_id()
    }
}

impl ErrorEvent for EnclaveEvent {
    type Error = EnclaveError;
    type ErrorType = EnclaveErrorType;
    fn as_error(&self) -> Option<&Self::Error> {
        match self {
            EnclaveEvent::EnclaveError { data, .. } => Some(data),
            _ => None,
        }
    }

    fn from_error(err_type: Self::ErrorType, error: anyhow::Error) -> Self {
        EnclaveEvent::from(EnclaveError::new(err_type, error.to_string().as_str()))
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
            EnclaveEvent::PublishDocumentRequested { id, .. } => id,
            EnclaveEvent::CiphernodeSelected { id, .. } => id,
            EnclaveEvent::CiphernodeAdded { id, .. } => id,
            EnclaveEvent::CiphernodeRemoved { id, .. } => id,
            EnclaveEvent::TicketBalanceUpdated { id, .. } => id,
            EnclaveEvent::CommitteePublished { id, .. } => id,
            EnclaveEvent::PlaintextOutputPublished { id, .. } => id,
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
            EnclaveEvent::CommitteePublished { data, .. } => Some(data.e3_id),
            EnclaveEvent::PlaintextOutputPublished { data, .. } => Some(data.e3_id),
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
            EnclaveEvent::PublishDocumentRequested { data, .. } => format!("{}", data),
            EnclaveEvent::CiphernodeSelected { data, .. } => format!("{}", data),
            EnclaveEvent::CiphernodeAdded { data, .. } => format!("{}", data),
            EnclaveEvent::CiphernodeRemoved { data, .. } => format!("{}", data),
            EnclaveEvent::TicketBalanceUpdated { data, .. } => format!("{:?}", data),
            EnclaveEvent::CommitteePublished { data, .. } => format!("{:?}", data),
            EnclaveEvent::PlaintextOutputPublished { data, .. } => format!("{:?}", data),
            EnclaveEvent::E3RequestComplete { data, .. } => format!("{}", data),
            EnclaveEvent::EnclaveError { data, .. } => format!("{:?}", data),
            EnclaveEvent::Shutdown { data, .. } => format!("{:?}", data),
            EnclaveEvent::TestEvent { data, .. } => format!("{:?}", data),
            // _ => "<omitted>".to_string(),
        }
    }
}

impl_from_event!(
    KeyshareCreated,
    E3Requested,
    PublicKeyAggregated,
    CiphertextOutputPublished,
    DecryptionshareCreated,
    PlaintextAggregated,
    PublishDocumentRequested,
    E3RequestComplete,
    CiphernodeSelected,
    CiphernodeAdded,
    CiphernodeRemoved,
    TicketBalanceUpdated,
    CommitteePublished,
    PlaintextOutputPublished,
    EnclaveError,
    Shutdown,
    TestEvent
);

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
