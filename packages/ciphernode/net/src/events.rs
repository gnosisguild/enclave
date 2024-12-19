use std::sync::Arc;

use actix::Message;
use libp2p::{
    gossipsub::{MessageId, PublishError},
    swarm::{dial_opts::DialOpts, ConnectionId, DialError},
};

use crate::correlation_id::CorrelationId;

pub enum NetworkPeerCommand {
    GossipPublish {
        topic: String,
        data: Vec<u8>,
        correlation_id: CorrelationId,
    },
    Dial(DialOpts),
}

#[derive(Message, Clone, Debug)]
#[rtype(result = "anyhow::Result<()>")]
pub enum NetworkPeerEvent {
    GossipData(Vec<u8>),
    GossipPublishError {
        // TODO: return an error here? DialError is not Clonable so we have
        // avoided passing it on
        correlation_id: CorrelationId,
        error: Arc<PublishError>,
    },
    GossipPublished {
        correlation_id: CorrelationId,
        message_id: MessageId,
    },
    DialError {
        error: Arc<DialError>,
    },
    ConnectionEstablished {
        connection_id: ConnectionId,
    },
    OutgoingConnectionError {
        connection_id: ConnectionId,
        error: Arc<DialError>,
    },
}
