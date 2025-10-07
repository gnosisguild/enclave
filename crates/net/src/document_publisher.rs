// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// TODO: Create DocumentPublisher actor
// 1. Accept EnclaveEvent::CiphernodeSelected and store selected e3_ids in a blume filter
// 1. Accept EnclaveEvent::PublishDocumentRequested
//    Take the payload and convert to NetCommand::PublishDocument
// 1. Accept NetEvent::DocumentPublishedNotification from NetInterface
//    Determine if we are keeping track of the given e3_id based on DocumentMeta
//    and the e3_id blume filter if so then issue a NetCommand::FetchDocument
// 1. Receive the document from NetEvent::FetchDocumentSucceeded and convert to
//    EnclaveEvent::DocumentReceived
// 1. Accept NetEvent::FetchDocumentFailed and attempt to retry

#![allow(dead_code)]

use crate::{
    events::{DocumentPublishedNotification, GossipData, NetCommand, NetEvent},
    Cid,
};
use actix::prelude::*;
use anyhow::Result;
use chrono::{DateTime, Utc};
use e3_events::{
    CorrelationId, DocumentMeta, EnclaveEvent, EventBus, PublishDocumentRequested, Subscribe,
};
use std::{sync::Arc, time::Instant};
use tokio::sync::{broadcast, mpsc};
use tracing::error;

pub struct DocumentPublisher {
    bus: Addr<EventBus<EnclaveEvent>>,
    tx: mpsc::Sender<NetCommand>,
    rx: Arc<broadcast::Receiver<NetEvent>>,
    topic: String,
}

impl DocumentPublisher {
    /// Create a new NetEventTranslator actor
    pub fn new(
        bus: &Addr<EventBus<EnclaveEvent>>,
        tx: &mpsc::Sender<NetCommand>,
        rx: &Arc<broadcast::Receiver<NetEvent>>,
        topic: impl Into<String>,
    ) -> Self {
        Self {
            bus: bus.clone(),
            tx: tx.clone(),
            rx: rx.clone(),
            topic: topic.into(),
        }
    }

    /// This is needed to create simulation libp2p event routers
    pub fn is_document_publisher_event(event: &EnclaveEvent) -> bool {
        // Add a list of events with paylods for the DHT
        match event {
            EnclaveEvent::PublishDocumentRequested { .. } => true,
            _ => false,
        }
    }

    pub fn setup(
        bus: &Addr<EventBus<EnclaveEvent>>,
        tx: &mpsc::Sender<NetCommand>,
        rx: &Arc<broadcast::Receiver<NetEvent>>,
        topic: impl Into<String>,
    ) -> Addr<Self> {
        let mut events = rx.resubscribe();
        let addr = Self::new(bus, tx, rx, topic).start();
        // Listen on all events
        bus.do_send(Subscribe::new("*", addr.clone().recipient()));

        // Forward gossip data from NetEvent
        tokio::spawn({
            let addr = addr.clone();
            async move {
                while let Ok(event) = events.recv().await {
                    match event {
                        NetEvent::GossipData(GossipData::DocumentPublishedNotification(data)) => {
                            addr.do_send(data)
                        }
                        _ => (),
                    }
                }
            }
        });

        addr
    }
}

impl Actor for DocumentPublisher {
    type Context = actix::Context<Self>;
}

impl Handler<EnclaveEvent> for DocumentPublisher {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            EnclaveEvent::PublishDocumentRequested { data, .. } => ctx.notify(data),
            _ => (),
        }
    }
}

impl Handler<PublishDocumentRequested> for DocumentPublisher {
    type Result = ResponseFuture<()>;
    fn handle(&mut self, msg: PublishDocumentRequested, _: &mut Self::Context) -> Self::Result {
        let tx = self.tx.clone();
        let msg = msg.clone();
        let rx = self.rx.clone();
        let topic = self.topic.clone();
        Box::pin(async move {
            match handle_publish_document_requested(tx, rx, msg, topic).await {
                Ok(_) => (),
                Err(e) => {
                    error!(error=?e, "Could not handle publish document requested");
                }
            }
        })
    }
}

impl Handler<DocumentPublishedNotification> for DocumentPublisher {
    type Result = ();
    fn handle(
        &mut self,
        msg: DocumentPublishedNotification,
        ctx: &mut Self::Context,
    ) -> Self::Result {
    }
}

pub fn datetime_to_instant_from_now(target: DateTime<Utc>) -> Option<Instant> {
    let now_datetime = Utc::now();
    let now_instant = Instant::now();

    if target <= now_datetime {
        return None; // Already expired
    }

    let duration = target.signed_duration_since(now_datetime);
    let std_duration = duration.to_std().ok()?;
    now_instant.checked_add(std_duration)
}

/// Called when we receive a PublishDocumentRequested event
pub async fn handle_publish_document_requested(
    tx: mpsc::Sender<NetCommand>,
    rx: Arc<broadcast::Receiver<NetEvent>>,
    event: PublishDocumentRequested,
    topic: impl Into<String>,
) -> Result<()> {
    let value = event.value;
    let key = Cid::from_content(&value);
    let expires = datetime_to_instant_from_now(event.meta.expires_at);
    put_record(tx.clone(), rx.clone(), expires, value, key.clone()).await?;

    broadcast_document_published_notification(
        tx,
        rx,
        DocumentPublishedNotification {
            meta: event.meta,
            key,
        },
        topic,
    )
    .await?;

    Ok(())
}

/// Called when we receive a notification from the net_interface
pub fn handle_document_published_notification(event: DocumentPublishedNotification) -> Result<()> {
    Ok(())
}

async fn put_record(
    net_cmds: mpsc::Sender<NetCommand>,
    net_events: Arc<broadcast::Receiver<NetEvent>>,
    expires: Option<Instant>,
    value: Vec<u8>,
    key: Cid,
) -> Result<()> {
    let id = CorrelationId::new();
    net_cmds
        .send(NetCommand::DhtPutRecord {
            correlation_id: id,
            expires,
            value,
            key,
        })
        .await?;

    let mut rx = net_events.resubscribe();
    loop {
        match rx.recv().await {
            Ok(NetEvent::DhtPutRecordSucceeded { correlation_id, .. }) if correlation_id == id => {
                return Ok(());
            }
            Ok(NetEvent::DhtPutRecordError {
                correlation_id,
                error,
            }) if correlation_id == id => {
                return Err(anyhow::anyhow!("DHT put record failed: {:?}", error));
            }
            Ok(_) => continue, // Ignore events with non-matching IDs or other events
            Err(broadcast::error::RecvError::Lagged(_)) => continue, // Receiver fell behind, keep trying
            Err(e) => return Err(e.into()), // Channel closed or other error
        }
    }
}

async fn broadcast_document_published_notification(
    net_cmds: mpsc::Sender<NetCommand>,
    net_events: Arc<broadcast::Receiver<NetEvent>>,
    payload: DocumentPublishedNotification,
    topic: impl Into<String>,
) -> Result<()> {
    let id = CorrelationId::new();

    net_cmds
        .send(NetCommand::GossipPublish {
            topic: topic.into(),
            correlation_id: id,
            data: GossipData::DocumentPublishedNotification(payload),
        })
        .await?;
    let mut rx = net_events.resubscribe();
    loop {
        match rx.recv().await {
            Ok(NetEvent::GossipPublished { correlation_id, .. }) if correlation_id == id => {
                return Ok(());
            }
            Ok(NetEvent::GossipPublishError {
                correlation_id,
                error,
            }) if correlation_id == id => {
                return Err(anyhow::anyhow!("GossipPublished failed: {:?}", error));
            }
            Ok(_) => continue, // Ignore events with non-matching IDs or other events
            Err(broadcast::error::RecvError::Lagged(_)) => continue, // Receiver fell behind, keep trying
            Err(e) => return Err(e.into()), // Channel closed or other error
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, time::Duration};

    use super::*;
    use crate::events::NetCommand;
    use anyhow::{bail, Result};
    use e3_events::{
        DocumentMeta, E3id, EnclaveEvent, EventBus, EventBusConfig, PublishDocumentRequested,
    };
    use tokio::{
        sync::{broadcast, mpsc},
        time::timeout,
    };

    #[actix::test]
    async fn test_publishes_document() -> Result<()> {
        let bus = EventBus::<EnclaveEvent>::new(EventBusConfig { deduplicate: true }).start();
        let (net_cmd_tx, mut net_cmd_rx) = mpsc::channel(100);
        let (net_evt_tx, net_evt_rx) = broadcast::channel(100);
        let net_evt_rx = Arc::new(net_evt_rx);

        DocumentPublisher::setup(&bus, &net_cmd_tx, &net_evt_rx, "topic");

        let value = b"I am a special document".to_vec();
        let expires_at = Utc::now() + chrono::Duration::days(1);

        bus.do_send(EnclaveEvent::from(PublishDocumentRequested {
            meta: DocumentMeta::new(E3id::new("1243", 1), vec![], expires_at),
            value: value.clone(),
        }));

        let Some(NetCommand::DhtPutRecord {
            correlation_id,
            expires,
            value: msg_value,
            key,
        }) = timeout(Duration::from_secs(1), net_cmd_rx.recv())
            .await
            .expect("did not receive DhtPutRecord")
        else {
            bail!("msg not as expected");
        };

        // Fake DHT put the record
        let mut mykad: HashMap<Cid, Vec<u8>> = HashMap::new();
        mykad.insert(key.clone(), msg_value.clone());

        // Everything went well
        net_evt_tx.send(NetEvent::DhtPutRecordSucceeded {
            correlation_id,
            key,
        })?;

        // Expect a DocumentPublishedNotification
        let Some(NetCommand::GossipPublish {
            topic,
            data: GossipData::DocumentPublishedNotification(notification),
            ..
        }) = timeout(Duration::from_secs(1), net_cmd_rx.recv())
            .await
            .expect("did not receive GossipPublish")
        else {
            bail!("msg not as expected");
        };

        assert_eq!(topic, "topic");
        assert_eq!(notification.meta.e3_id, E3id::new("1243", 1));

        // put together enclave event -> DocumentPublished
        //
        // DocumentPublished holds DocumentMeta and Cid
        // Nodes can then use the DocumentMeta to deternmine if they want to request the document
        // assert_eq enclave_event.to_bytes()
        assert_eq!(
            mykad.get(&notification.key),
            Some(&b"I am a special document".to_vec()),
            "value was not correct"
        );
        assert!(
            is_between(expires.unwrap(), days_from_now(0), days_from_now(1)),
            "Expiry was not set"
        );

        Ok(())
    }

    pub fn is_between(instant: Instant, start: Instant, end: Instant) -> bool {
        let (min, max) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        instant >= min && instant <= max
    }

    pub fn days_from_now(days: u64) -> Instant {
        Instant::now() + Duration::from_secs(60 * 60 * 24 * days)
    }
}
