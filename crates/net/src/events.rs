// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::Message;
use e3_events::{CorrelationId, DocumentMeta};
use libp2p::{
    gossipsub::{MessageId, PublishError},
    swarm::{dial_opts::DialOpts, ConnectionId, DialError},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

type Cid = Vec<u8>;
type DocumentBytes = Vec<u8>;

/// NetInterface Commands are sent to the network peer over a mspc channel
pub enum NetCommand {
    /// Publish message to gossipsub
    GossipPublish {
        topic: String,
        data: Vec<u8>,
        correlation_id: CorrelationId,
    },
    /// Dial peer
    Dial(DialOpts),
    /// Command to PublishDocument to Kademlia
    PublishDocument {
        meta: DocumentMeta,
        value: Vec<u8>,
        cid: Cid,
    },
    /// Fetch Document from Kademlia
    FetchDocument {
        correlation_id: CorrelationId,
        cid: Cid,
    },
}

/// NetEvents are broadcast over a broadcast channel to whom ever wishes to listen
#[derive(Message, Clone, Debug)]
#[rtype(result = "anyhow::Result<()>")]
pub enum NetEvent {
    /// Bytes have been broadcast over the network
    GossipData(Vec<u8>),
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

    /// This node received a document pubilshed notification
    DocumentPublishedNotification(DocumentPublishedNotification),
    /// This node received a document from a Kademlia Request
    FetchDocumentSucceeded {
        cid: Cid,
        correlation_id: CorrelationId,
        value: DocumentBytes,
    },
    /// There was an error receiving the document
    FetchDocumentFailed {
        correlation_id: CorrelationId,
        error: (), // TODO: Use Arc<Specific Kademlia Error>
    },
}

/// Payload that is dispatched as a net -> net gossip event from Kademlia. This event signals that
/// a document was published and that this node might be interested in it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DocumentPublishedNotification {
    meta: DocumentMeta,
    cid: Cid,
}

impl DocumentPublishedNotification {
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
}
