use std::sync::Arc;

use actix::Message;
use libp2p::{
    gossipsub::{MessageId, PublishError},
    swarm::{dial_opts::DialOpts, ConnectionId, DialError},
};

use events::{Event, EventId, Subscribe};

use crate::correlation_id::CorrelationId;

/// NetworkPeer Commands are sent to the network peer over a mspc channel
#[derive(Message)]
#[rtype(result = "()")]
pub enum NetworkPeerCommand {
    GossipPublish {
        topic: String,
        data: Vec<u8>,
        correlation_id: CorrelationId,
    },
    Dial(DialOpts),
}

/// NetworkPeerEvents are broadcast over a broadcast channel to whom ever wishes to listen
#[derive(Message, Clone, Debug)]
#[rtype(result = "()")]
pub enum NetworkPeerEvent {
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
    DialError {
        connection_id: ConnectionId,
        error: Arc<DialError>,
    },
    /// A connection was established to a peer
    ConnectionEstablished { connection_id: ConnectionId },
    /// There was an error creating a connection
    OutgoingConnectionError {
        connection_id: ConnectionId,
        error: Arc<DialError>,
    },
}

impl NetworkPeerEvent {
    pub fn event_type(&self) -> String {
        let s = format!("{:?}", self);
        extract_event_name(&s).to_string()
    }
}

impl Event for NetworkPeerEvent {
    type Id = String;

    fn event_type(&self) -> String {
        let s = format!("{:?}", self);
        extract_event_name(&s).to_string()
    }

    fn event_id(&self) -> Self::Id {
        "network_peer_event".to_string()
    }
}

fn extract_event_name(s: &str) -> &str {
    let bytes = s.as_bytes();
    for (i, &item) in bytes.iter().enumerate() {
        if item == b' ' || item == b'(' {
            return &s[..i];
        }
    }
    s
}
