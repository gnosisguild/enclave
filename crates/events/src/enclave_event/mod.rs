// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

mod ciphernode_added;
mod ciphernode_removed;
mod ciphernode_selected;
mod ciphertext_output_published;
mod committee_finalize_requested;
mod committee_finalized;
mod committee_published;
mod committee_requested;
mod compute_request;
mod configuration_updated;
mod decryptionshare_created;
mod die;
mod e3_request_complete;
mod e3_requested;
mod enclave_error;
mod keyshare_created;
mod operator_activation_changed;
mod plaintext_aggregated;
mod plaintext_output_published;
mod publickey_aggregated;
mod publish_document;
mod shutdown;
mod test_event;
mod threshold_share_created;
mod ticket_balance_updated;
mod ticket_generated;
mod ticket_submitted;

pub use ciphernode_added::*;
pub use ciphernode_removed::*;
pub use ciphernode_selected::*;
pub use ciphertext_output_published::*;
pub use committee_finalize_requested::*;
pub use committee_finalized::*;
pub use committee_published::*;
pub use committee_requested::*;
pub use compute_request::*;
pub use configuration_updated::*;
pub use decryptionshare_created::*;
pub use die::*;
pub use e3_request_complete::*;
pub use e3_requested::*;
pub use enclave_error::*;
pub use keyshare_created::*;
pub use operator_activation_changed::*;
pub use plaintext_aggregated::*;
pub use plaintext_output_published::*;
pub use publickey_aggregated::*;
pub use publish_document::*;
pub use shutdown::*;
use strum::IntoStaticStr;
pub use test_event::*;
pub use threshold_share_created::*;
pub use ticket_balance_updated::*;
pub use ticket_generated::*;
pub use ticket_submitted::*;

use crate::{
    traits::{ErrorEvent, Event, EventConstructorWithTimestamp},
    E3id, EventId,
};
use actix::Message;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    fmt::{self},
    hash::Hash,
};

/// Macro to help define From traits for EnclaveEventData
macro_rules! impl_into_event_data {
    ($($variant:ident),*) => {
        $(
            impl From<$variant> for EnclaveEventData {
                fn from(data:$variant) -> Self {
                    EnclaveEventData::$variant(data)
                }
            }
        )*
    };
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, IntoStaticStr, Serialize, Deserialize)]
pub enum EnclaveEventData {
    KeyshareCreated(KeyshareCreated),
    E3Requested(E3Requested),
    PublicKeyAggregated(PublicKeyAggregated),
    CiphertextOutputPublished(CiphertextOutputPublished),
    DecryptionshareCreated(DecryptionshareCreated),
    PlaintextAggregated(PlaintextAggregated),
    PublishDocumentRequested(PublishDocumentRequested),
    CiphernodeSelected(CiphernodeSelected),
    CiphernodeAdded(CiphernodeAdded),
    CiphernodeRemoved(CiphernodeRemoved),
    TicketBalanceUpdated(TicketBalanceUpdated),
    ConfigurationUpdated(ConfigurationUpdated),
    OperatorActivationChanged(OperatorActivationChanged),
    CommitteePublished(CommitteePublished),
    CommitteeRequested(CommitteeRequested),
    CommitteeFinalizeRequested(CommitteeFinalizeRequested),
    CommitteeFinalized(CommitteeFinalized),
    TicketGenerated(TicketGenerated),
    TicketSubmitted(TicketSubmitted),
    PlaintextOutputPublished(PlaintextOutputPublished),
    EnclaveError(EnclaveError),
    E3RequestComplete(E3RequestComplete),
    Shutdown(Shutdown),
    DocumentReceived(DocumentReceived),
    ThresholdShareCreated(ThresholdShareCreated),
    /// This is a test event to use in testing
    TestEvent(TestEvent),
}

impl EnclaveEventData {
    pub fn event_type(&self) -> String {
        let name: &'static str = self.into();
        name.to_string()
    }
}

pub trait SeqState: Clone + std::fmt::Debug + 'static {
    type Seq: Unpin
        + Sync
        + Send
        + Serialize
        + DeserializeOwned
        + Clone
        + PartialEq
        + Eq
        + Hash
        + std::fmt::Debug;
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Unstored;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Stored;

impl SeqState for Unstored {
    type Seq = ();
}

impl SeqState for Stored {
    type Seq = u64;
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct EnclaveEvent<S: SeqState = Stored> {
    id: EventId,
    payload: EnclaveEventData,
    seq: S::Seq,
}

impl<S> EnclaveEvent<S>
where
    S: SeqState,
{
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }

    pub fn get_id(&self) -> EventId {
        self.into()
    }
}

impl EnclaveEvent<Unstored> {
    pub fn into_stored(self, seq: u64) -> EnclaveEvent<Stored> {
        EnclaveEvent::<Stored> {
            id: self.id,
            payload: self.payload,
            seq,
        }
    }
}

#[cfg(feature = "test-helpers")]
impl EnclaveEvent<Stored> {
    pub fn new_stored_event(data: EnclaveEventData, time: u128, seq: u64) -> Self {
        EnclaveEvent::<Unstored>::new_with_timestamp(data, time).into_stored(seq)
    }
}

impl<S: SeqState> Event for EnclaveEvent<S> {
    type Id = EventId;
    type Data = EnclaveEventData;

    fn event_type(&self) -> String {
        self.payload.event_type()
    }

    fn event_id(&self) -> Self::Id {
        self.get_id()
    }

    fn get_data(&self) -> &EnclaveEventData {
        &self.payload
    }
    fn into_data(self) -> EnclaveEventData {
        self.payload
    }
}

impl ErrorEvent for EnclaveEvent<Unstored> {
    type ErrType = EType;
    type FromError = anyhow::Error;

    fn from_error(err_type: Self::ErrType, msg: impl Into<Self::FromError>) -> Self {
        let payload = EnclaveError::new(err_type, msg);
        let id = EventId::hash(&payload);
        EnclaveEvent {
            payload: payload.into(),
            id,
            seq: (),
        }
    }
}

impl<S: SeqState> From<EnclaveEvent<S>> for EventId {
    fn from(value: EnclaveEvent<S>) -> Self {
        value.id
    }
}

impl<S: SeqState> From<&EnclaveEvent<S>> for EventId {
    fn from(value: &EnclaveEvent<S>) -> Self {
        value.id.clone()
    }
}

impl<S: SeqState> EnclaveEvent<S> {
    pub fn get_e3_id(&self) -> Option<E3id> {
        match self.payload {
            EnclaveEventData::KeyshareCreated(ref data) => Some(data.e3_id.clone()),
            EnclaveEventData::E3Requested(ref data) => Some(data.e3_id.clone()),
            EnclaveEventData::PublicKeyAggregated(ref data) => Some(data.e3_id.clone()),
            EnclaveEventData::CiphertextOutputPublished(ref data) => Some(data.e3_id.clone()),
            EnclaveEventData::DecryptionshareCreated(ref data) => Some(data.e3_id.clone()),
            EnclaveEventData::PlaintextAggregated(ref data) => Some(data.e3_id.clone()),
            EnclaveEventData::CiphernodeSelected(ref data) => Some(data.e3_id.clone()),
            EnclaveEventData::ThresholdShareCreated(ref data) => Some(data.e3_id.clone()),
            EnclaveEventData::CommitteePublished(ref data) => Some(data.e3_id.clone()),
            EnclaveEventData::CommitteeRequested(ref data) => Some(data.e3_id.clone()),
            EnclaveEventData::CommitteeFinalizeRequested(ref data) => Some(data.e3_id.clone()),
            EnclaveEventData::PlaintextOutputPublished(ref data) => Some(data.e3_id.clone()),
            EnclaveEventData::CommitteeFinalized(ref data) => Some(data.e3_id.clone()),
            EnclaveEventData::TicketGenerated(ref data) => Some(data.e3_id.clone()),
            EnclaveEventData::TicketSubmitted(ref data) => Some(data.e3_id.clone()),
            _ => None,
        }
    }
}

impl_into_event_data!(
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
    ConfigurationUpdated,
    OperatorActivationChanged,
    CommitteePublished,
    CommitteeRequested,
    CommitteeFinalizeRequested,
    CommitteeFinalized,
    TicketGenerated,
    TicketSubmitted,
    PlaintextOutputPublished,
    EnclaveError,
    Shutdown,
    TestEvent,
    DocumentReceived,
    ThresholdShareCreated
);

impl TryFrom<&EnclaveEvent<Stored>> for EnclaveError {
    type Error = anyhow::Error;
    fn try_from(value: &EnclaveEvent) -> Result<Self, Self::Error> {
        value.clone().try_into()
    }
}

impl TryFrom<EnclaveEvent<Stored>> for EnclaveError {
    type Error = anyhow::Error;
    fn try_from(value: EnclaveEvent<Stored>) -> Result<Self, Self::Error> {
        if let EnclaveEventData::EnclaveError(data) = value.payload.clone() {
            Ok(data)
        } else {
            return Err(anyhow::anyhow!("Not an enclave error {:?}", value));
        }
    }
}

impl<S: SeqState> fmt::Display for EnclaveEvent<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!("{:?}", self))
    }
}

impl EventConstructorWithTimestamp for EnclaveEvent<Unstored> {
    fn new_with_timestamp(data: Self::Data, _ts: u128) -> Self {
        let payload = data.into();
        let id = EventId::hash(&payload);
        // hcl.receive(remote_ts)?;
        EnclaveEvent {
            id,
            payload,
            seq: (),
        }
    }
}
