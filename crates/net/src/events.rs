// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::Cid;
use actix::Message;
use anyhow::{Context, Result};
use e3_events::{CorrelationId, DocumentMeta};
use libp2p::{
    gossipsub::{MessageId, PublishError},
    swarm::{dial_opts::DialOpts, ConnectionId, DialError},
};
use serde::{Deserialize, Serialize};
use std::{hash::Hash, sync::Arc, time::Instant};

/// Incoming/Outgoing GossipData. We disambiguate on concerns relative to the net package.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum GossipData {
    GossipBytes(Vec<u8>), // Serialized EnclaveEvent
    DocumentPublishedNotification(DocumentPublishedNotification),
}

impl GossipData {
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).context("Could not serialize GossipData")
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes).context("Could not deserialize GossipData")
    }
}

/// NetInterface Commands are sent to the network peer over a mspc channel
pub enum NetCommand {
    /// Publish message to gossipsub
    GossipPublish {
        topic: String,
        data: GossipData,
        correlation_id: CorrelationId,
    },
    /// Dial peer
    Dial(DialOpts),
    /// Command to PublishDocument to Kademlia
    DhtPutRecord {
        correlation_id: CorrelationId,
        expires: Option<Instant>,
        value: Vec<u8>,
        key: Cid,
    },
    /// Fetch Document from Kademlia
    DhtGetRecord {
        correlation_id: CorrelationId,
        key: Cid,
    },
}

/// NetEvents are broadcast over a broadcast channel to whom ever wishes to listen
#[derive(Message, Clone, Debug)]
#[rtype(result = "anyhow::Result<()>")]
pub enum NetEvent {
    /// Bytes have been broadcast over the network
    GossipData(GossipData),
    /// There was an Error publishing bytes over the network
    GossipPublishError {
        correlation_id: CorrelationId,
        error: Arc<PublishError>,
    },
    /// Data was successfully published over the network as far as we know.
    GossipPublished {
        correlation_id: CorrelationId,
        message_id: MessageId,
    },
    /// There was an error Dialing a peer
    DialError { error: Arc<DialError> },
    /// A connection was established to a peer
    ConnectionEstablished { connection_id: ConnectionId },
    /// There was an error creating a connection
    OutgoingConnectionError {
        connection_id: ConnectionId,
        error: Arc<DialError>,
    },
    /// This node received a document from a Kademlia Request
    DhtGetRecordSucceeded {
        key: Cid,
        correlation_id: CorrelationId,
        value: Vec<u8>,
    },
    /// This node received a document from a Kademlia Request
    DhtPutRecordSucceeded {
        key: Cid,
        correlation_id: CorrelationId,
    },
    /// There was an error receiving the document
    DhtGetRecordError {
        correlation_id: CorrelationId,
        error: DhtGetRecordError,
    },
    /// There was an error putting the document
    DhtPutRecordError {
        correlation_id: CorrelationId,
        error: DhtPutRecordError,
    },
}

#[derive(Clone, Debug)]
pub enum DhtGetRecordError {
    NotFound,
    QuorumFailed,
    Timeout,
}

#[derive(Clone, Debug)]
pub enum DhtPutRecordError {
    QuorumFailed,
    Timeout,
}

/// Payload that is dispatched as a net -> net gossip event from Kademlia. This event signals that
/// a document was published and that this node might be interested in it.
#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct DocumentPublishedNotification {
    pub meta: DocumentMeta,
    pub key: Cid,
}

impl DocumentPublishedNotification {
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).context("Could not serialize DocumentPublishedNotification")
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes).context("Could not deserialize DocumentPublishedNotification")
    }
}
