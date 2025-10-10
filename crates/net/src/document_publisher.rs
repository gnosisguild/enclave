// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// TODO: Create DocumentPublisher actor
// - [ ] Accept EnclaveEvent::CiphernodeSelected and store selected e3_ids in a blume filter
// - [x] Accept EnclaveEvent::PublishDocumentRequested
// - [x] Take the payload and convert to NetCommand::DhtPutRecord
// - [-] Accept NetEvent::GossipData(GossipData::DocumentPublishedNotification) from NetInterface
//       Determine if we are keeping track of the given e3_id based on DocumentMeta
//       and the e3_id hashset if so then issue a NetCommand::DhtGetRecord
// - [ ] Receive the document from NetEvent::FetchDocumentSucceeded and convert to
//        EnclaveEvent::DocumentReceived
// - [ ] Accept NetEvent::DhtGetRecordError and attempt to retry with exponential backoff

#![allow(dead_code)]

use crate::{
    events::{DocumentPublishedNotification, GossipData, NetCommand, NetEvent},
    Cid,
};
use actix::prelude::*;
use anyhow::Result;
use chrono::{DateTime, Utc};
use e3_events::{
    CiphernodeSelected, CorrelationId, E3RequestComplete, E3id, EnclaveEvent, EventBus,
    PublishDocumentRequested, Subscribe,
};
use std::{collections::HashSet, sync::Arc, time::Instant};
use tokio::sync::{broadcast, mpsc};
use tracing::error;

/// DocumentPublisher is an actor that monitors events from both the NetInterface and the Enclave
/// EventBus in order to manage document publishing interactions. In particular this involves the
/// interactions of publishing a document and listening for notifications, determining if the node
/// is interested in a specific document and fetching the document to broadcast on the local event
/// bus
pub struct DocumentPublisher {
    /// Enclave EventBus
    bus: Addr<EventBus<EnclaveEvent>>,
    /// NetCommand sender to forward commands to the NetInterface
    tx: mpsc::Sender<NetCommand>,
    /// NetEvent receiver to resubscribe for events from the NetInterface. This is in an Arc so
    /// that we do not do excessive resubscribes without actually listening for events.
    rx: Arc<broadcast::Receiver<NetEvent>>,
    /// The gossipsub broadcast topic
    topic: String,
    /// Set of E3ids we are interested in
    ids: HashSet<E3id>,
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
            ids: HashSet::new(),
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

    /// Setup the DocumentPublisher and start listening for GossipEvents
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

    fn handle_ciphernode_selected(&mut self, event: CiphernodeSelected) -> Result<()> {
        self.ids.insert(event.e3_id);
        Ok(())
    }

    fn handle_e3_request_complete(&mut self, event: E3RequestComplete) -> Result<()> {
        self.ids.remove(&event.e3_id);
        Ok(())
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
            EnclaveEvent::CiphernodeSelected { data, .. } => ctx.notify(data),
            EnclaveEvent::E3RequestComplete { data, .. } => ctx.notify(data),
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

impl Handler<CiphernodeSelected> for DocumentPublisher {
    type Result = ();
    fn handle(&mut self, msg: CiphernodeSelected, _ctx: &mut Self::Context) -> Self::Result {
        match self.handle_ciphernode_selected(msg) {
            Ok(_) => (),
            Err(e) => {
                error!("{e}")
            }
        }
    }
}

impl Handler<E3RequestComplete> for DocumentPublisher {
    type Result = ();
    fn handle(&mut self, msg: E3RequestComplete, _ctx: &mut Self::Context) -> Self::Result {
        match self.handle_e3_request_complete(msg) {
            Ok(_) => (),
            Err(e) => {
                error!("{e}")
            }
        }
    }
}

impl Handler<DocumentPublishedNotification> for DocumentPublisher {
    type Result = ();
    fn handle(
        &mut self,
        msg: DocumentPublishedNotification,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        match handle_document_published_notification(msg) {
            Ok(_) => (),
            Err(e) => {
                error!("{e}")
            }
        }
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
pub fn handle_document_published_notification(_: DocumentPublishedNotification) -> Result<()> {
    // tbc..
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

    // NOTE: The following pattern we should generalize
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

    // NOTE: The following pattern we should generalize
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
        CiphernodeSelected, DocumentMeta, E3id, EnclaveEvent, EventBus, EventBusConfig,
        PublishDocumentRequested,
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
        let e3_id = E3id::new("1243", 1);

        // 1. Send a request to publish
        bus.do_send(EnclaveEvent::from(PublishDocumentRequested {
            meta: DocumentMeta::new(e3_id, vec![], expires_at),
            value: value.clone(),
        }));

        // 2. Document publisher should have asked the NetInterface to put the doc on Kademlia
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

        // 3. Report that everything went well
        net_evt_tx.send(NetEvent::DhtPutRecordSucceeded {
            correlation_id,
            key,
        })?;

        // 4. Expect a DocumentPublishedNotification to have been emitted
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

    #[actix::test]
    async fn test_notified_of_document() -> Result<()> {
        let bus = EventBus::<EnclaveEvent>::new(EventBusConfig { deduplicate: true }).start();
        let (net_cmd_tx, mut net_cmd_rx) = mpsc::channel(100);
        let (net_evt_tx, net_evt_rx) = broadcast::channel(100);
        let net_evt_rx = Arc::new(net_evt_rx);

        DocumentPublisher::setup(&bus, &net_cmd_tx, &net_evt_rx, "topic");

        let value = b"I am a special document".to_vec();
        let expires_at = Utc::now() + chrono::Duration::days(1);
        let e3_id = E3id::new("1243", 1);
        let cid = Cid::from_content(&value);

        // 1. Ensure the publisher is interested in the id by receiving CiphernodeSelected
        bus.do_send(EnclaveEvent::from(CiphernodeSelected {
            e3_id: e3_id.clone(),
            threshold_m: 5, // TODO: this will change with the merging of #660
        }));

        // 2. Dispatch a NetEvent from the NetInterface signaling that a document was published
        net_evt_tx.send(NetEvent::GossipData(
            GossipData::DocumentPublishedNotification(DocumentPublishedNotification {
                key: Cid::from_content(&b"wrong document".to_vec()),
                meta: DocumentMeta::new(e3_id.clone(), vec![], expires_at),
            }),
        ))?;

        // 3. Nothing happens...
        let result = timeout(Duration::from_secs(1), net_cmd_rx.recv()).await;
        assert!(result.is_err(), "Expected timeout but received a message");

        // 4. Dispatch a NetEvent from the NetInterface signaling that a document we ARE interested
        //    in was published
        net_evt_tx.send(NetEvent::GossipData(
            GossipData::DocumentPublishedNotification(DocumentPublishedNotification {
                key: cid.clone(),
                meta: DocumentMeta::new(e3_id, vec![], expires_at),
            }),
        ))?;

        // 5. Expect that DocumentPublisher will make a DhtGetRecord request
        let Some(NetCommand::DhtGetRecord {
            key,
            correlation_id,
        }) = timeout(Duration::from_secs(1), net_cmd_rx.recv())
            .await
            .expect("did not receive DhtGetRecord")
        else {
            bail!("msg not as expected");
        };

        assert_eq!(key, cid);

        // 6. Forward the document
        net_evt_tx.send(NetEvent::DhtGetRecordSucceeded {
            key: cid,
            correlation_id, // same correlation_id
            value,
        })?;

        // XXX: Need some of the testing tools from #660 to wait for the bus event

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
