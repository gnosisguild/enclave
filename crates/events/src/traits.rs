// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::{Message, Recipient};
use std::fmt::Display;
use std::hash::Hash;

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

    fn from_error(err_type: Self::ErrType, error: impl Into<Self::FromError>) -> Self;
}

/// An EventFactory creates events
pub trait EventFactory<E: Event> {
    /// Create a new event from the given event data, apply a local HLC timestamp.
    ///
    /// This method should be used for events that have originated locally.
    fn event_from(&self, data: impl Into<E::Data>) -> E;
    /// Create a new event from the given event data, apply the given remote HLC time to ensure correct
    /// event ordering.
    ///
    /// This method should be used for events that originated from remote sources.
    fn event_from_remote_source(&self, data: impl Into<E::Data>, ts: u128) -> E;
}

/// An ErrorFactory creates errors.
pub trait ErrorFactory<E: ErrorEvent> {
    /// Create an error event from the given error.
    fn event_from_error(&self, err_type: E::ErrType, error: impl Into<E::FromError>) -> E;
}

/// An EventPublisher publishes events on it's internal EventBus
pub trait EventPublisher<E: Event> {
    /// Create a new event from the given event data, apply a local HLC timestamp and publish it
    /// to the event bus.
    ///
    /// This method should be used for events that have originated locally.
    fn publish(&self, data: impl Into<E::Data>);
    /// Create a new event from the given event data, apply the given remote HLC time to ensure correct
    /// event ordering and publish it.
    ///
    /// This method should be used for events that originated from remote sources.
    fn publish_from_remote(&self, data: impl Into<E::Data>, ts: u128);
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

pub trait CompositeEvent: ErrorEvent + EventConstructorWithTimestamp {}

impl<E> CompositeEvent for E where E: Sized + Event + ErrorEvent + EventConstructorWithTimestamp {}
