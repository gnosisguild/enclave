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
mod enable_effects;
mod enclave_error;
mod encryption_key_collection_failed;
mod encryption_key_created;
mod keyshare_created;
mod net_sync_events_received;
mod operator_activation_changed;
mod outgoing_sync_requested;
mod plaintext_aggregated;
mod plaintext_output_published;
mod publickey_aggregated;
mod publish_document;
mod shutdown;
mod sync_effect;
mod sync_end;
mod sync_start;
mod test_event;
mod threshold_share_collection_failed;
mod threshold_share_created;
mod ticket_balance_updated;
mod ticket_generated;
mod ticket_submitted;
mod typed_event;

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
use e3_utils::{colorize, colorize_event_ids, Color};
pub use enable_effects::*;
pub use enclave_error::*;
pub use encryption_key_collection_failed::*;
pub use encryption_key_created::*;
pub use keyshare_created::*;
pub use net_sync_events_received::*;
pub use operator_activation_changed::*;
pub use outgoing_sync_requested::*;
pub use plaintext_aggregated::*;
pub use plaintext_output_published::*;
pub use publickey_aggregated::*;
pub use publish_document::*;
pub use shutdown::*;
use strum::IntoStaticStr;
pub use sync_effect::*;
pub use sync_end::*;
pub use sync_start::*;
pub use test_event::*;
pub use threshold_share_collection_failed::*;
pub use threshold_share_created::*;
pub use ticket_balance_updated::*;
pub use ticket_generated::*;
pub use ticket_submitted::*;
pub use typed_event::*;

use crate::{
    event_context::{AggregateId, EventContext},
    traits::{ErrorEvent, Event, EventConstructorWithTimestamp, EventContextAccessors},
    E3id, EventContextSeq, EventId, WithAggregateId,
};
use actix::Message;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    fmt::{self},
    hash::Hash,
};

// In crates/events/src/enclave_event/mod.rs

/// Macro to generate EventType enum and implement From traits
macro_rules! impl_event_types {
    ($($variant:ident),* $(,)?) => {
        // Generate the EventType enum
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum EventType {
            /// Wildcard - matches all events
            All,
            $($variant,)*
        }

        impl EventType {
            /// Get the string representation of this event type
            pub fn as_str(&self) -> &'static str {
                match self {
                    EventType::All => "*",
                    $(EventType::$variant => stringify!($variant),)*
                }
            }

            /// Parse an EventType from a string
            pub fn from_str(s: &str) -> Option<Self> {
                match s {
                    "*" => Some(EventType::All),
                    $(stringify!($variant) => Some(EventType::$variant),)*
                    _ => None,
                }
            }

            /// Get all event types (excluding All wildcard)
            pub fn all_types() -> Vec<EventType> {
                vec![
                    $(EventType::$variant,)*
                ]
            }
        }

        impl fmt::Display for EventType {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.as_str())
            }
        }

        impl From<EventType> for String {
            fn from(event_type: EventType) -> Self {
                event_type.as_str().to_string()
            }
        }

        impl From<&EventType> for String {
            fn from(event_type: &EventType) -> Self {
                event_type.as_str().to_string()
            }
        }

        // Implement From<EnclaveEventData> for EventType
        impl From<&EnclaveEventData> for EventType {
            fn from(data: &EnclaveEventData) -> Self {
                match data {
                    $(EnclaveEventData::$variant(_) => EventType::$variant,)*
                }
            }
        }

        impl From<EnclaveEventData> for EventType {
            fn from(data: EnclaveEventData) -> Self {
                (&data).into()
            }
        }

        $(
            impl From<$variant> for EnclaveEventData {
                fn from(data: $variant) -> Self {
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
    EncryptionKeyCreated(EncryptionKeyCreated),
    EncryptionKeyCollectionFailed(EncryptionKeyCollectionFailed),
    ThresholdShareCollectionFailed(ThresholdShareCollectionFailed),
    ComputeRequest(ComputeRequest),           // ComputeRequested
    ComputeResponse(ComputeResponse),         // ComputeResponseReceived
    ComputeRequestError(ComputeRequestError), // ComputeRequestFailed
    NetSyncEventsReceived(NetSyncEventsReceived),
    HistoricalEvmSyncStart(HistoricalEvmSyncStart),
    HistoricalNetSyncStart(HistoricalNetSyncStart),
    SyncEffect(SyncEffect),
    SyncEnd(SyncEnd),
    EnableEffects(EnableEffects),
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
pub struct Unsequenced;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Sequenced;

impl SeqState for Unsequenced {
    type Seq = ();
}

impl SeqState for Sequenced {
    type Seq = u64;
}

#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
#[serde(bound(
    serialize = "S: SeqState, S::Seq: Serialize",
    deserialize = "S: SeqState, S::Seq: DeserializeOwned"
))]
pub struct EnclaveEvent<S: SeqState = Sequenced> {
    payload: EnclaveEventData,
    ctx: EventContext<S>,
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

    pub fn split(self) -> (EnclaveEventData, u128) {
        (self.payload, self.ctx.ts())
    }

    pub fn into_components(self) -> (EnclaveEventData, EventContext<S>) {
        (self.payload, self.ctx)
    }

    pub fn get_ctx(&self) -> &EventContext<S> {
        &self.ctx
    }
}

impl<S: SeqState> EventContextAccessors for EnclaveEvent<S> {
    fn causation_id(&self) -> EventId {
        self.ctx.causation_id()
    }
    fn origin_id(&self) -> EventId {
        self.ctx.origin_id()
    }
    fn ts(&self) -> u128 {
        self.ctx.ts()
    }
    fn id(&self) -> EventId {
        self.ctx.id()
    }
    fn aggregate_id(&self) -> AggregateId {
        self.ctx.aggregate_id()
    }
    fn block(&self) -> Option<u64> {
        self.ctx.block()
    }
}

impl EventContextSeq for EnclaveEvent<Sequenced> {
    fn seq(&self) -> u64 {
        self.ctx.seq()
    }
}

impl EnclaveEvent<Sequenced> {
    pub fn clone_unsequenced(&self) -> EnclaveEvent<Unsequenced> {
        let ts = self.ts();
        let block = self.block();
        let data = self.clone().into_data();
        EnclaveEvent::new_with_timestamp(data, Some(self.ctx.clone()), ts, block)
    }

    pub fn to_typed_event<T>(&self, data: T) -> TypedEvent<T> {
        let ctx: EventContext<Sequenced> = self.get_ctx().clone();
        TypedEvent::new(data, ctx)
    }
}

impl EnclaveEvent<Unsequenced> {
    pub fn into_sequenced(self, seq: u64) -> EnclaveEvent<Sequenced> {
        EnclaveEvent::<Sequenced> {
            payload: self.payload,
            ctx: self.ctx.sequence(seq),
        }
    }
}

#[cfg(feature = "test-helpers")]
impl EnclaveEvent<Sequenced> {
    /// test-helpers only utility function to create a new sequenced event
    pub fn new_stored_event(data: EnclaveEventData, time: u128, seq: u64) -> Self {
        EnclaveEvent::<Unsequenced>::new_with_timestamp(data, None, time, None).into_sequenced(seq)
    }

    /// test-helpers only utility function to create a new sequenced event
    pub fn from_data_ec(data: EnclaveEventData, ec: EventContext<Sequenced>) -> Self {
        EnclaveEvent::<Unsequenced>::new_with_timestamp(data, Some(ec.clone()), ec.ts(), ec.block())
            .into_sequenced(ec.seq())
    }

    /// test-helpers only utility function to remove time information from an event
    pub fn strip_ts(&self) -> EnclaveEvent {
        EnclaveEvent::new_stored_event(self.get_data().clone(), 0, self.seq())
    }
}

impl<S: SeqState> Event for EnclaveEvent<S> {
    type Id = EventId;
    type Data = EnclaveEventData;

    fn event_id(&self) -> Self::Id {
        self.ctx.id()
    }

    fn event_type(&self) -> String {
        self.payload.event_type()
    }

    fn get_data(&self) -> &EnclaveEventData {
        &self.payload
    }

    fn into_data(self) -> EnclaveEventData {
        self.payload
    }
}

impl ErrorEvent for EnclaveEvent<Unsequenced> {
    type ErrType = EType;
    type FromError = anyhow::Error;

    fn from_error(
        err_type: Self::ErrType,
        msg: impl Into<Self::FromError>,
        ts: u128,
        caused_by: Option<EventContext<Sequenced>>,
    ) -> anyhow::Result<Self> {
        let payload = EnclaveError::new(err_type, msg);
        let id = EventId::hash(&payload);
        let aggregate_id = AggregateId::new(0); // Error events use default aggregate_id

        let ctx = caused_by
            .map(|cause| EventContext::from_cause(id, cause, ts, aggregate_id, None))
            .unwrap_or_else(|| EventContext::new_origin(id, ts, aggregate_id, None));

        Ok(EnclaveEvent {
            payload: payload.into(),
            ctx,
        })
    }
}

impl<S: SeqState> From<EnclaveEvent<S>> for EventId {
    fn from(value: EnclaveEvent<S>) -> Self {
        value.ctx.id()
    }
}

impl<S: SeqState> From<&EnclaveEvent<S>> for EventId {
    fn from(value: &EnclaveEvent<S>) -> Self {
        value.ctx.id()
    }
}

impl EnclaveEventData {
    pub fn get_e3_id(&self) -> Option<E3id> {
        match self {
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
            EnclaveEventData::EncryptionKeyCreated(ref data) => Some(data.e3_id.clone()),
            EnclaveEventData::ComputeResponse(ref data) => Some(data.e3_id.clone()),
            EnclaveEventData::TestEvent(ref data) => data.e3_id.clone(),
            _ => None,
        }
    }
}

impl WithAggregateId for EnclaveEventData {
    fn get_aggregate_id(&self) -> AggregateId {
        let chain_id = self.get_e3_id().map(|e3_id| e3_id.chain_id());
        AggregateId::from_chain_id(chain_id)
    }
}

impl<S: SeqState> EnclaveEvent<S> {
    pub fn get_e3_id(&self) -> Option<E3id> {
        self.payload.get_e3_id()
    }
}

impl<S: SeqState> WithAggregateId for EnclaveEvent<S> {
    fn get_aggregate_id(&self) -> AggregateId {
        self.payload.get_aggregate_id()
    }
}

impl_event_types!(
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
    ThresholdShareCreated,
    EncryptionKeyCreated,
    EncryptionKeyCollectionFailed,
    ThresholdShareCollectionFailed,
    ComputeRequest,
    ComputeResponse,
    ComputeRequestError,
    NetSyncEventsReceived,
    HistoricalEvmSyncStart,
    HistoricalNetSyncStart,
    SyncEffect,
    SyncEnd,
    EnableEffects
);

impl TryFrom<&EnclaveEvent<Sequenced>> for EnclaveError {
    type Error = anyhow::Error;
    fn try_from(value: &EnclaveEvent) -> Result<Self, Self::Error> {
        value.clone().try_into()
    }
}

impl TryFrom<EnclaveEvent<Sequenced>> for EnclaveError {
    type Error = anyhow::Error;
    fn try_from(value: EnclaveEvent<Sequenced>) -> Result<Self, Self::Error> {
        if let EnclaveEventData::EnclaveError(data) = value.payload.clone() {
            Ok(data)
        } else {
            return Err(anyhow::anyhow!("Not an enclave error {:?}", value));
        }
    }
}
impl From<EnclaveEvent<Sequenced>> for EventContext<Sequenced> {
    fn from(value: EnclaveEvent) -> Self {
        (&value).into()
    }
}
impl From<&EnclaveEvent<Sequenced>> for EventContext<Sequenced> {
    fn from(value: &EnclaveEvent) -> Self {
        value.ctx.clone()
    }
}

// Add convenience method to EnclaveEvent
impl<S: SeqState> EnclaveEvent<S> {
    /// Get the EventType enum for this event
    pub fn event_type_enum(&self) -> EventType {
        (&self.payload).into()
    }
}

impl<S: SeqState> fmt::Display for EnclaveEvent<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let t = self.event_type();
        let colorized_debug = colorize_event_ids(self);

        let s = match t.as_str() {
            "EnclaveError" => format!("{} {}", colorize(t, Color::Red), colorized_debug),
            _ => format!("{} {}", colorize(t, Color::Cyan), colorized_debug),
        };
        f.write_str(&s)
    }
}

impl EventConstructorWithTimestamp for EnclaveEvent<Unsequenced> {
    fn new_with_timestamp(
        data: Self::Data,
        caused_by: Option<EventContext<Sequenced>>,
        ts: u128,
        block: Option<u64>,
    ) -> Self {
        let payload: EnclaveEventData = data.into();
        let id = EventId::hash(&payload);
        let aggregate_id = payload.get_aggregate_id();
        EnclaveEvent {
            payload,
            ctx: caused_by
                .map(|cause| EventContext::from_cause(id, cause, ts, aggregate_id, block))
                .unwrap_or_else(|| EventContext::new_origin(id, ts, aggregate_id, block)),
        }
    }
}
