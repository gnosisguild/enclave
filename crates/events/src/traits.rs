// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Message, Recipient};
use anyhow::Result;
use std::fmt::Display;
use std::hash::Hash;

use crate::{EnclaveEvent, Unsequenced};

/// Trait that must be implemented by events used with EventBus
pub trait Event:
    Message<Result = ()> + Clone + Display + Send + Sync + Unpin + Sized + 'static
{
    type Id: Hash + Eq + Clone + Unpin + Send + Sync + Display;

    /// Payload for the Event
    type Data;

    fn event_type(&self) -> String;
    fn event_id(&self) -> Self::Id;
    fn get_data(&self) -> &Self::Data;
    fn into_data(self) -> Self::Data;
}

/// Trait for events that contain an error
pub trait ErrorEvent: Event {
    /// Error type associated with this event
    type ErrType;
    type FromError;

    fn from_error(
        err_type: Self::ErrType,
        error: impl Into<Self::FromError>,
        ts: u128,
    ) -> Result<Self>;
}

/// An EventFactory creates events
pub trait EventFactory<E: Event> {
    /// Create a new event from the given event data, apply a local HLC timestamp.
    ///
    /// This method should be used for events that have originated locally.
    fn event_from(&self, data: impl Into<E::Data>) -> Result<E>;
    /// Create a new event from the given event data, apply the given remote HLC time to ensure correct
    /// event ordering.
    ///
    /// This method should be used for events that originated from remote sources.
    fn event_from_remote_source(&self, data: impl Into<E::Data>, ts: u128) -> Result<E>;
}

/// An ErrorFactory creates errors.
pub trait ErrorFactory<E: ErrorEvent> {
    /// Create an error event from the given error.
    fn event_from_error(&self, err_type: E::ErrType, error: impl Into<E::FromError>) -> Result<E>;
}

/// An EventPublisher publishes events on it's internal EventBus
pub trait EventPublisher<E: Event> {
    /// Create a new event from the given event data, apply a local HLC timestamp and publish it
    /// to the event bus.
    ///
    /// This method should be used for events that have originated locally.
    fn publish(&self, data: impl Into<E::Data>) -> Result<()>;
    /// Create a new event from the given event data, apply the given remote HLC time to ensure correct
    /// event ordering and publish it.
    ///
    /// This method should be used for events that originated from remote sources.
    fn publish_from_remote(&self, data: impl Into<E::Data>, ts: u128) -> Result<()>;
    /// Dispatch the given event without applying any HLC transformation.
    fn naked_dispatch(&self, event: E);
}

/// Trait for dispatching errors to an inner event bus
pub trait ErrorDispatcher<E: ErrorEvent> {
    /// Dispatch the error to the event bus.
    fn err(&self, err_type: E::ErrType, error: impl Into<E::FromError>);
}

/// Trait to subscribe to events
pub trait EventSubscriber<E: Event> {
    /// Subscribe the recipient to events matching the given event type
    fn subscribe(&self, event_type: &str, recipient: Recipient<E>);
    /// Subscribe the recipient to events matching any of the given event types
    fn subscribe_all(&self, event_types: &[&str], recipient: Recipient<E>);
}

/// Trait to create an event with a timestamp from its associated type data
pub trait EventConstructorWithTimestamp: Event + Sized {
    /// Create an event passing attaching a specific timestamp.
    fn new_with_timestamp(data: Self::Data, ts: u128) -> Self;
}

pub trait CompositeEvent: EventConstructorWithTimestamp {}

impl<E> CompositeEvent for E where E: Sized + Event + EventConstructorWithTimestamp {}

/// SequenceIndex is the index for each sequence which we can lookup based on HLC timestamp
pub trait SequenceIndex: Unpin + 'static {
    /// Insert a sequence offset at the given timestamp
    fn insert(&mut self, key: u128, value: u64) -> Result<()>;
    /// Get the sequence offset for the given timestamp
    fn get(&self, key: u128) -> Result<Option<u64>>;
    /// Get the first sequence offset before the given timestamp
    fn seek_for_prev(&self, key: u128) -> Result<Option<u64>>;
}

/// Store and retrieve events from a write ahead log
pub trait EventLog: Unpin + 'static {
    /// Append an event to the log, returning its sequence number
    fn append(&mut self, event: &EnclaveEvent<Unsequenced>) -> Result<u64>;
    /// Read all events starting from the given sequence number (inclusive)
    fn read_from(
        &self,
        from: u64,
    ) -> Box<
        dyn Iterator<Item = std::result::Result<(u64, EnclaveEvent<Unsequenced>), anyhow::Error>>,
    >;
}
