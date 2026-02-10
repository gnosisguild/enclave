// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::Cid;
use actix::Message;
use anyhow::{bail, Context, Result};
use e3_events::{
    AggregateId, CorrelationId, DocumentMeta, EnclaveEvent, EventContextAccessors, EventSource,
    Sequenced, Unsequenced,
};
use e3_utils::{ArcBytes, OnceTake};
use libp2p::{
    gossipsub::{MessageId, PublishError, TopicHash},
    kad::{store, GetRecordError, PutRecordError},
    request_response::{InboundRequestId, ResponseChannel},
    swarm::{dial_opts::DialOpts, ConnectionId, DialError},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    hash::Hash,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{broadcast, mpsc};
use tracing::{error, info};

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

impl TryFrom<EnclaveEvent<Sequenced>> for GossipData {
    type Error = anyhow::Error;
    fn try_from(value: EnclaveEvent<Sequenced>) -> Result<Self, Self::Error> {
        let bytes = value
            .clone_unsequenced() // Note serializing UNSEQUENCED
            .to_bytes()
            .context("Could not convert event to bytes for serialization!")?;
        Ok(GossipData::GossipBytes(bytes))
    }
}

impl TryFrom<GossipData> for EnclaveEvent<Unsequenced> {
    type Error = anyhow::Error;
    fn try_from(value: GossipData) -> Result<Self, Self::Error> {
        let GossipData::GossipBytes(bytes) = value else {
            bail!("GossipData was not the GossipBytes variant");
        };

        Ok(EnclaveEvent::from_bytes(&bytes)?.with_source(EventSource::Net))
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyncRequestValue {
    pub since: HashMap<AggregateId, u128>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyncResponseValue {
    pub events: Vec<GossipData>,
    pub ts: u128,
}

#[derive(Message, Clone, Debug)]
#[rtype("()")]
pub struct SyncRequestReceived {
    pub request_id: InboundRequestId,
    pub value: SyncRequestValue,
    pub channel: OnceTake<ResponseChannel<SyncResponseValue>>,
}

#[derive(Message, Clone, Debug)]
#[rtype("()")]
pub struct OutgoingSyncRequestSucceeded {
    pub value: SyncResponseValue,
    pub correlation_id: CorrelationId,
}

#[derive(Debug, Clone)]
pub struct OutgoingSyncRequestFailed;

/// NetInterface Commands are sent to the network peer over a mspc channel
#[derive(Debug)]
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
        value: ArcBytes,
        key: Cid,
    },
    /// Fetch Document from Kademlia
    DhtGetRecord {
        correlation_id: CorrelationId,
        key: Cid,
    },
    /// Shutdown signal
    Shutdown,
    /// Called from the syning node to request libp2p events from a random peer node starting
    /// from the given timestamp.
    OutgoingSyncRequest {
        correlation_id: CorrelationId,
        value: SyncRequestValue,
    },
    /// Send libp2p events back to a peer that requested a sync.
    SyncResponse {
        value: SyncResponseValue,
        channel: OnceTake<ResponseChannel<SyncResponseValue>>,
    },
}

impl NetCommand {
    pub fn correlation_id(&self) -> Option<CorrelationId> {
        use NetCommand as N;
        match self {
            N::DhtPutRecord { correlation_id, .. } => Some(*correlation_id),
            N::DhtGetRecord { correlation_id, .. } => Some(*correlation_id),
            N::GossipPublish { correlation_id, .. } => Some(*correlation_id),
            N::OutgoingSyncRequest { correlation_id, .. } => Some(*correlation_id),
            _ => None,
        }
    }
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
    DialError {
        error: Arc<DialError>,
    },
    /// A connection was established to a peer
    ConnectionEstablished {
        connection_id: ConnectionId,
    },
    /// There was an error creating a connection
    OutgoingConnectionError {
        connection_id: ConnectionId,
        error: Arc<DialError>,
    },
    /// This node received a document from a Kademlia Request
    DhtGetRecordSucceeded {
        key: Cid,
        correlation_id: CorrelationId,
        value: ArcBytes,
    },
    /// This node received a document from a Kademlia Request
    DhtPutRecordSucceeded {
        key: Cid,
        correlation_id: CorrelationId,
    },
    /// There was an error receiving the document
    DhtGetRecordError {
        correlation_id: CorrelationId,
        error: GetRecordError,
    },
    /// There was an error putting the document
    DhtPutRecordError {
        correlation_id: CorrelationId,
        error: PutOrStoreError,
    },
    /// GossipSubscribed
    GossipSubscribed {
        count: usize,
        topic: TopicHash,
    },
    /// A peer node is requesting gossipsub events since the given timestamp.
    /// Use the provided channel to send a `SyncResponse
    SyncRequestReceived(SyncRequestReceived),
    /// Received gossipsub events from a peer in response to a `SyncRequest`.
    OutgoingSyncRequestSucceeded(OutgoingSyncRequestSucceeded),
    OutgoingSyncRequestFailed(OutgoingSyncRequestFailed),
    AllPeersDialed,
}

#[derive(Clone, Debug)]
pub enum PutOrStoreError {
    PutRecordError(PutRecordError),
    StoreError(store::Error),
}

impl NetEvent {
    pub fn correlation_id(&self) -> Option<CorrelationId> {
        use NetEvent as N;
        match self {
            N::GossipPublished { correlation_id, .. } => Some(*correlation_id),
            N::GossipPublishError { correlation_id, .. } => Some(*correlation_id),
            N::DhtGetRecordError { correlation_id, .. } => Some(*correlation_id),
            N::DhtGetRecordSucceeded { correlation_id, .. } => Some(*correlation_id),
            N::DhtPutRecordError { correlation_id, .. } => Some(*correlation_id),
            N::DhtPutRecordSucceeded { correlation_id, .. } => Some(*correlation_id),
            _ => None,
        }
    }
}

/// Payload that is dispatched as a net -> net gossip event from Kademlia. This event signals that
/// a document was published and that this node might be interested in it.
#[derive(Message, Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[rtype(result = "()")]
pub struct DocumentPublishedNotification {
    pub meta: DocumentMeta,
    pub key: Cid,
    pub ts: u128,
}

impl DocumentPublishedNotification {
    pub fn new(meta: DocumentMeta, key: Cid, ts: u128) -> Self {
        Self { meta, key, ts }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        bincode::serialize(self).context("Could not serialize DocumentPublishedNotification")
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        bincode::deserialize(bytes).context("Could not deserialize DocumentPublishedNotification")
    }
}

/// Generic helper for the command-response pattern with correlation IDs
pub async fn call_and_await_response<F, R>(
    net_cmds: mpsc::Sender<NetCommand>,
    net_events: Arc<broadcast::Receiver<NetEvent>>,
    command: NetCommand,
    matcher: F,
    timeout: Duration,
) -> Result<R>
where
    F: Fn(&NetEvent) -> Option<Result<R>>,
{
    // Resubscribe first to avoid missing events
    let mut rx = net_events.resubscribe();

    // Extract correlation_id from command
    let Some(id) = command.correlation_id() else {
        return Err(anyhow::anyhow!(format!(
            "Command must have a correlation_id but this does not: {:?}",
            command
        )));
    };

    // We don't have access to this later and we cannot clone command
    let debug_cmd = format!("{:?}", command);

    // Send the command to NetInterface
    info!(
        "call_and_await_response: SENDING command {:?} with timeout {:?}",
        command, timeout
    );
    net_cmds.send(command).await?;

    let result = tokio::time::timeout(timeout, async {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    info!("RECEIVED and event {:?}", event);
                    // Only process events matching our correlation ID
                    if event.correlation_id() == Some(id) {
                        if let Some(result) = matcher(&event) {
                            return result;
                        } // None means unexpected event type, keep waiting
                        info!("matcher failed for event {:?}! skipping...", event);
                    } else {
                        // Ignore events with non-matching IDs
                        info!(
                            "correlation failed for event {:?} against correlation={:?}! skipping...",
                            event, id
                        );
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(e) => {
                    error!("RECEIVED an error from rx: {:?} returning error", e);
                    return Err(e.into());
                }
            }
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!(format!("Timed out waiting for response from {}", debug_cmd)))?;
    result
}

pub async fn await_event<F, R>(
    net_events: &Arc<broadcast::Receiver<NetEvent>>,
    matcher: F,
    timeout: Duration,
) -> Result<R>
where
    F: Fn(&NetEvent) -> Option<R>,
{
    let mut rx = net_events.resubscribe();

    let result = tokio::time::timeout(timeout, async {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    if let Some(result) = matcher(&event) {
                        return Ok(result);
                    }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(e) => return Err(e.into()),
            }
        }
    })
    .await
    .map_err(|_| anyhow::anyhow!(format!("Timed out waiting for event")))?;
    result
}

#[cfg(test)]
mod tests {
    use e3_events::{
        EnclaveEvent, EventConstructorWithTimestamp, EventSource, Sequenced, TestEvent, Unsequenced,
    };

    use super::GossipData;

    #[test]
    fn test_enclave_event_gossip_lifecycle() -> anyhow::Result<()> {
        // event is created locally
        let event: EnclaveEvent<Unsequenced> = EnclaveEvent::new_with_timestamp(
            TestEvent::new("fish", 42).into(),
            None,
            31415,
            None,
            EventSource::local,
        );

        // event is sequenced after bus.publish() adds a sequence number
        let event: EnclaveEvent<Sequenced> = event.into_sequenced(90210);

        // event is broadcast
        let gossip_data: GossipData = event.try_into()?;

        let GossipData::GossipBytes(_) = gossip_data else {
            panic!("events must only be serialized to GossipBytes");
        };

        // received gossip data from libp2p convert to unsequenced event
        let event: EnclaveEvent<Unsequenced> = gossip_data.try_into()?;
        let (data, ts) = event.split();

        assert_eq!(data, TestEvent::new("fish", 42).into());
        assert_eq!(ts, 31415);

        Ok(())
    }
}
