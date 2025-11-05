// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{
    events::{
        call_and_await_response, DocumentPublishedNotification, GossipData, NetCommand, NetEvent,
    },
    Cid,
};
use actix::prelude::*;
use anyhow::Result;
use chrono::{DateTime, Utc};
use e3_events::{
    BusError, CiphernodeSelected, CorrelationId, DocumentKind, DocumentMeta, DocumentReceived,
    E3RequestComplete, E3id, EnclaveErrorType, EnclaveEvent, EventBus, PartyId,
    PublishDocumentRequested, Subscribe, ThresholdShareCreated,
};
use e3_utils::retry::{retry_with_backoff, to_retry};
use e3_utils::ArcBytes;
use futures::TryFutureExt;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, error, info, warn};

const KADEMLIA_PUT_TIMEOUT: Duration = Duration::from_secs(30);
const KADEMLIA_GET_TIMEOUT: Duration = Duration::from_secs(30);
const KADEMLIA_BROADCAST_TIMEOUT: Duration = Duration::from_secs(30);

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
    ids: HashMap<E3id, PartyId>,
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
            ids: HashMap::new(),
        }
    }

    /// This is needed to create simulation libp2p event routers
    pub fn is_document_publisher_event(event: &EnclaveEvent) -> bool {
        // Add a list of events with paylods for the DHT
        match event {
            EnclaveEvent::PublishDocumentRequested { .. } => true,
            EnclaveEvent::ThresholdShareCreated { .. } => true,
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

        // Convert events
        EventConverter::setup(bus);

        // Listen on all events
        bus.do_send(Subscribe::new("*", addr.clone().recipient()));

        // Forward gossip data from NetEvent
        tokio::spawn({
            debug!("Spawning event receive loop!");
            let addr = addr.clone();
            async move {
                while let Ok(event) = events.recv().await {
                    debug!("Received event {:?}", event);
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
        let CiphernodeSelected {
            e3_id, party_id, ..
        } = event;
        self.ids.insert(e3_id, party_id);
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
        let bus = self.bus.clone();
        let topic = self.topic.clone();
        Box::pin(async move {
            match handle_publish_document_requested(tx, rx, msg, topic).await {
                Ok(_) => (),
                Err(e) => {
                    error!(error=?e, "Could not handle publish document requested");
                    bus.err(EnclaveErrorType::IO, e)
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
    type Result = ResponseFuture<()>;
    fn handle(
        &mut self,
        msg: DocumentPublishedNotification,
        _: &mut Self::Context,
    ) -> Self::Result {
        let ids = self.ids.clone();
        let bus = self.bus.clone();
        let tx = self.tx.clone();
        let msg = msg.clone();
        let rx = self.rx.clone();

        Box::pin(async move {
            match handle_document_published_notification(tx, rx, bus.clone(), ids, msg).await {
                Ok(_) => (),
                Err(e) => {
                    error!(error=?e, "Could not handle document published notification");
                    bus.err(EnclaveErrorType::IO, e);
                }
            }
        })
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

    retry_with_backoff(
        || {
            put_record(tx.clone(), rx.clone(), expires, value.clone(), key.clone())
                .map_err(to_retry)
        },
        4,
        1000,
    )
    .await?;
    let notification = DocumentPublishedNotification::new(event.meta, key);
    broadcast_document_published_notification(tx, rx, notification, topic).await?;
    Ok(())
}

/// Called when we receive a notification from the net_interface
pub async fn handle_document_published_notification(
    net_cmds: mpsc::Sender<NetCommand>,
    net_events: Arc<broadcast::Receiver<NetEvent>>,
    bus: Addr<EventBus<EnclaveEvent>>,
    ids: HashMap<E3id, PartyId>,
    event: DocumentPublishedNotification,
) -> Result<()> {
    let e3_id = &event.meta.e3_id;
    let Some(party_id) = ids.get(e3_id) else {
        debug!("Node not interested in id {}", e3_id);
        return Ok(());
    };

    if !event.meta.matches(party_id) {
        return Ok(());
    }

    debug!(
        "interested in document {:?} with party_id={:?}",
        event, party_id
    );

    let value = retry_with_backoff(
        || get_record(net_cmds.clone(), net_events.clone(), event.key.clone()).map_err(to_retry),
        4,
        1000,
    )
    .await?;

    debug!("Sending received event...");
    bus.do_send(EnclaveEvent::from(DocumentReceived {
        meta: event.meta,
        value,
    }));

    Ok(())
}

/// Call DhtPutRecord Command on the NetInterface and handle the results
async fn put_record(
    net_cmds: mpsc::Sender<NetCommand>,
    net_events: Arc<broadcast::Receiver<NetEvent>>,
    expires: Option<Instant>,
    value: ArcBytes,
    key: Cid,
) -> Result<()> {
    let id = CorrelationId::new();
    call_and_await_response(
        net_cmds,
        net_events,
        NetCommand::DhtPutRecord {
            correlation_id: id,
            expires,
            value,
            key,
        },
        |event| match event {
            NetEvent::DhtPutRecordSucceeded { .. } => Some(Ok(())),
            NetEvent::DhtPutRecordError { error, .. } => {
                Some(Err(anyhow::anyhow!("DHT put record failed: {:?}", error)))
            }
            _ => None,
        },
        KADEMLIA_PUT_TIMEOUT,
    )
    .await
}

/// Call DhtPutRecord Command on the NetInterface and handle the results
async fn get_record(
    net_cmds: mpsc::Sender<NetCommand>,
    net_events: Arc<broadcast::Receiver<NetEvent>>,
    key: Cid,
) -> Result<ArcBytes> {
    let id = CorrelationId::new();
    call_and_await_response(
        net_cmds,
        net_events,
        NetCommand::DhtGetRecord {
            correlation_id: id,
            key,
        },
        |event| match event {
            NetEvent::DhtGetRecordSucceeded { value, .. } => Some(Ok(value.clone())),
            NetEvent::DhtGetRecordError { error, .. } => {
                Some(Err(anyhow::anyhow!("DHT get record failed: {:?}", error)))
            }
            _ => None,
        },
        KADEMLIA_GET_TIMEOUT,
    )
    .await
}

/// Broadcasts document published notification on NetInterface
async fn broadcast_document_published_notification(
    net_cmds: mpsc::Sender<NetCommand>,
    net_events: Arc<broadcast::Receiver<NetEvent>>,
    payload: DocumentPublishedNotification,
    topic: impl Into<String>,
) -> Result<()> {
    let id = CorrelationId::new();
    call_and_await_response(
        net_cmds,
        net_events,
        NetCommand::GossipPublish {
            topic: topic.into(),
            correlation_id: id,
            data: GossipData::DocumentPublishedNotification(payload),
        },
        |event| match event {
            NetEvent::GossipPublished { .. } => Some(Ok(())),
            NetEvent::GossipPublishError { error, .. } => {
                Some(Err(anyhow::anyhow!("GossipPublished failed: {:?}", error)))
            }
            _ => None,
        },
        KADEMLIA_BROADCAST_TIMEOUT,
    )
    .await
}

/// Convert between ThresholdShareCreated and DocumentPublished events
pub struct EventConverter {
    bus: Addr<EventBus<EnclaveEvent>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum ReceivableDocument {
    ThresholdShareCreated(ThresholdShareCreated),
}

impl ReceivableDocument {
    pub fn get_e3_id(&self) -> &E3id {
        match self {
            ReceivableDocument::ThresholdShareCreated(d) => &d.e3_id,
        }
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
}

impl EventConverter {
    pub fn new(bus: &Addr<EventBus<EnclaveEvent>>) -> Self {
        Self { bus: bus.clone() }
    }

    pub fn setup(bus: &Addr<EventBus<EnclaveEvent>>) -> Addr<Self> {
        let addr = Self::new(bus).start();
        bus.do_send(Subscribe::new("ThresholdShareCreated", addr.clone().into()));
        bus.do_send(Subscribe::new("DocumentReceived", addr.clone().into()));
        addr
    }

    /// Local node created a threshold share. Send it as a published document
    pub fn handle_threshold_share_created(&self, msg: ThresholdShareCreated) -> Result<()> {
        // If this is received from elsewhere
        if msg.external {
            return Ok(());
        }
        let receivable = ReceivableDocument::ThresholdShareCreated(msg);
        let value = ArcBytes::from_bytes(receivable.to_bytes()?);
        let meta = DocumentMeta::new(
            receivable.get_e3_id().clone(),
            DocumentKind::TrBFV,
            vec![],
            None,
        );

        self.bus
            .do_send(EnclaveEvent::from(PublishDocumentRequested::new(
                meta, value,
            )));
        Ok(())
    }
    /// Received document externally
    pub fn handle_document_received(&self, msg: DocumentReceived) -> Result<()> {
        warn!("Converting DocumentReceived...");
        let receivable = ReceivableDocument::from_bytes(&msg.value.extract_bytes())?;
        let event = EnclaveEvent::from(match receivable {
            ReceivableDocument::ThresholdShareCreated(evt) => ThresholdShareCreated {
                external: true,
                e3_id: evt.e3_id,
                share: evt.share,
            },
        });

        self.bus.do_send(event);
        Ok(())
    }
}

impl Actor for EventConverter {
    type Context = actix::Context<Self>;
}

impl Handler<EnclaveEvent> for EventConverter {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            EnclaveEvent::ThresholdShareCreated { data, .. } => ctx.notify(data),
            EnclaveEvent::DocumentReceived { data, .. } => ctx.notify(data),
            _ => (),
        }
    }
}

impl Handler<ThresholdShareCreated> for EventConverter {
    type Result = ();
    fn handle(&mut self, msg: ThresholdShareCreated, ctx: &mut Self::Context) -> Self::Result {
        match self.handle_threshold_share_created(msg) {
            Ok(_) => (),
            Err(err) => error!("{err}"),
        }
    }
}

impl Handler<DocumentReceived> for EventConverter {
    type Result = ();
    fn handle(&mut self, msg: DocumentReceived, ctx: &mut Self::Context) -> Self::Result {
        match self.handle_document_received(msg) {
            Ok(_) => (),
            Err(err) => error!("{err}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, num::NonZero, time::Duration};

    use super::*;
    use crate::events::NetCommand;
    use actix::Addr;
    use anyhow::{bail, Result};
    use e3_events::{
        CiphernodeSelected, DocumentKind, DocumentMeta, E3id, EnclaveError, EnclaveEvent, EventBus,
        EventBusConfig, GetEvents, HistoryCollector, PublishDocumentRequested, TakeEvents,
    };
    use libp2p::kad::{GetRecordError, PutRecordError, RecordKey};
    use tokio::{
        sync::{broadcast, mpsc},
        time::{sleep, timeout},
    };
    use tracing::subscriber::DefaultGuard;

    fn setup_test() -> (
        DefaultGuard,
        Addr<EventBus<EnclaveEvent>>,
        mpsc::Sender<NetCommand>,
        mpsc::Receiver<NetCommand>,
        broadcast::Sender<NetEvent>,
        Arc<broadcast::Receiver<NetEvent>>,
        Addr<HistoryCollector<EnclaveEvent>>,
        Addr<HistoryCollector<EnclaveEvent>>,
        Addr<DocumentPublisher>,
    ) {
        use tracing_subscriber::{fmt, EnvFilter};

        let subscriber = fmt()
            .with_env_filter(EnvFilter::new("debug"))
            .with_test_writer()
            .finish();

        let guard = tracing::subscriber::set_default(subscriber);

        let bus = EventBus::<EnclaveEvent>::new(EventBusConfig { deduplicate: true }).start();
        let (net_cmd_tx, net_cmd_rx) = mpsc::channel(100);
        let (net_evt_tx, net_evt_rx) = broadcast::channel(100);
        let net_evt_rx = Arc::new(net_evt_rx);
        let history = HistoryCollector::<EnclaveEvent>::new().start();
        let error = HistoryCollector::<EnclaveEvent>::new().start();
        bus.do_send(Subscribe::new("*", history.clone().recipient()));
        bus.do_send(Subscribe::new("EnclaveError", error.clone().recipient()));

        let publisher = DocumentPublisher::setup(&bus, &net_cmd_tx, &net_evt_rx, "topic");

        (
            guard, bus, net_cmd_tx, net_cmd_rx, net_evt_tx, net_evt_rx, history, error, publisher,
        )
    }

    #[actix::test]
    async fn test_publishes_document() -> Result<()> {
        let (_guard, bus, _net_cmd_tx, mut net_cmd_rx, net_evt_tx, _net_evt_rx, _, _, _) =
            setup_test();
        let value = ArcBytes::from_bytes(b"I am a special document".to_vec());
        let expires_at = Some(Utc::now() + chrono::Duration::days(1));
        let e3_id = E3id::new("1243", 1);

        // 1. Send a request to publish
        bus.do_send(EnclaveEvent::from(PublishDocumentRequested {
            meta: DocumentMeta::new(e3_id, DocumentKind::TrBFV, vec![], expires_at),
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
        mykad.insert(key.clone(), msg_value.extract_bytes());

        // 3. Report that everything went well
        net_evt_tx.send(NetEvent::DhtPutRecordSucceeded {
            correlation_id,
            key,
        })?;

        // 4. Expect a DocumentPublishedNotification to have been emitted
        let Some(NetCommand::GossipPublish {
            topic,
            correlation_id,
            data: GossipData::DocumentPublishedNotification(notification),
            ..
        }) = timeout(Duration::from_secs(1), net_cmd_rx.recv())
            .await
            .expect("did not receive GossipPublish")
        else {
            bail!("msg not as expected");
        };

        // 5. Report everything went well
        net_evt_tx.send(NetEvent::GossipPublished {
            correlation_id,
            message_id: libp2p::gossipsub::MessageId::new(&[1, 2, 3]),
        })?;

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
    async fn test_get_document_fails_with_exponential_backoff() -> Result<()> {
        let (_guard, bus, _net_cmd_tx, mut net_cmd_rx, net_evt_tx, _net_evt_rx, _, errors, _) =
            setup_test();

        let value = b"I am a special document".to_vec();
        let expires_at = Some(Utc::now() + chrono::Duration::days(1));
        let e3_id = E3id::new("1243", 1);
        let cid = Cid::from_content(&value);

        // 1. Ensure the publisher is interested in the id by receiving CiphernodeSelected
        bus.do_send(EnclaveEvent::from(CiphernodeSelected {
            e3_id: e3_id.clone(),
            threshold_m: 3,
            threshold_n: 5,
            ..CiphernodeSelected::default()
        }));

        net_evt_tx.send(NetEvent::GossipData(
            GossipData::DocumentPublishedNotification(DocumentPublishedNotification {
                key: cid.clone(),
                meta: DocumentMeta::new(e3_id, DocumentKind::TrBFV, vec![], expires_at),
            }),
        ))?;

        for _ in 0..4 {
            // Expect retry
            let Some(NetCommand::DhtGetRecord { correlation_id, .. }) =
                timeout(Duration::from_secs(15), net_cmd_rx.recv())
                    .await
                    .expect("did not receive DhtGetRecord")
            else {
                bail!("msg not as expected");
            };

            // Report failure
            net_evt_tx.send(NetEvent::DhtGetRecordError {
                correlation_id,
                error: GetRecordError::Timeout {
                    key: RecordKey::new(&cid),
                },
            })?;
        }

        // wait for events to settle
        let errors = errors.send(TakeEvents::new(1)).await?;
        let error: EnclaveError = errors.first().unwrap().try_into()?;
        assert_eq!(
            error.message,
            "Operation failed after 4 attempts. Last error: DHT get record failed: Timeout { key: Key(b\"\\xda-\\xe1\\xc0T\\x11$X\\x05\\xd1\\xd4\\xa6C\\x86\\x96\\xb7e\\xd9j\\x96\\x1bD\\xc8P#\\x0f\\\"\\xea A@b\") }"
        );

        Ok(())
    }

    #[actix::test]
    async fn test_publishes_document_fails_with_exponential_backoff() -> Result<()> {
        let (
            _guard,
            bus,
            _net_cmd_tx,
            mut net_cmd_rx,
            net_evt_tx,
            _net_evt_rx,
            _history,
            errors,
            _,
        ) = setup_test();
        let value = ArcBytes::from_bytes(b"I am a special document".to_vec());
        let expires_at = Some(Utc::now() + chrono::Duration::days(1));
        let e3_id = E3id::new("1243", 1);

        // Send a request to publish
        bus.do_send(EnclaveEvent::from(PublishDocumentRequested {
            meta: DocumentMeta::new(e3_id, DocumentKind::TrBFV, vec![], expires_at),
            value: value.clone(),
        }));

        for _ in 0..4 {
            // Expect retry
            let Some(NetCommand::DhtPutRecord { correlation_id, .. }) =
                timeout(Duration::from_secs(15), net_cmd_rx.recv())
                    .await
                    .expect("did not receive DhtPutRecord")
            else {
                bail!("msg not as expected");
            };

            // Report failure
            net_evt_tx.send(NetEvent::DhtPutRecordError {
                correlation_id,
                error: crate::events::PutOrStoreError::PutRecordError(
                    PutRecordError::QuorumFailed {
                        key: RecordKey::new(b"I got the secret"),
                        success: vec![],
                        quorum: NonZero::new(1).unwrap(),
                    },
                ),
            })?;
        }

        // Expect error to exist
        let errors = errors.send(TakeEvents::new(1)).await?;
        let error: EnclaveError = errors.first().unwrap().try_into()?;
        assert_eq!(
            error.message,
            "Operation failed after 4 attempts. Last error: DHT put record failed: PutRecordError(QuorumFailed { key: Key(b\"I got the secret\"), success: [], quorum: 1 })"
        );

        Ok(())
    }

    #[actix::test]
    async fn test_notified_of_document() -> Result<()> {
        let (_guard, bus, _net_cmd_tx, mut net_cmd_rx, net_evt_tx, _net_evt_rx, history, _, _) =
            setup_test();

        let value = ArcBytes::from_bytes(b"I am a special document".to_vec());
        let expires_at = Utc::now() + chrono::Duration::days(1);
        let e3_id = E3id::new("1243", 1);
        let cid = Cid::from_content(&value);

        // 1. Ensure the publisher is interested in the id by receiving CiphernodeSelected
        bus.do_send(EnclaveEvent::from(CiphernodeSelected {
            e3_id: e3_id.clone(),
            threshold_m: 3,
            threshold_n: 5,
            ..CiphernodeSelected::default()
        }));

        // 2. Dispatch a NetEvent from the NetInterface signaling that a document was published
        net_evt_tx.send(NetEvent::GossipData(
            GossipData::DocumentPublishedNotification(DocumentPublishedNotification {
                key: Cid::from_content(&b"wrong document".to_vec()),
                meta: DocumentMeta::new(
                    E3id::new("1111", 1),
                    DocumentKind::TrBFV,
                    vec![],
                    Some(expires_at),
                ),
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
                meta: DocumentMeta::new(e3_id, DocumentKind::TrBFV, vec![], Some(expires_at)),
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
            correlation_id,
            value,
        })?;

        // wait for events to settle
        sleep(Duration::from_millis(100)).await;

        // Check event was dispatched
        let events = history.send(GetEvents::new()).await?;
        let Some(EnclaveEvent::DocumentReceived {
            data: DocumentReceived { value: doc, .. },
            ..
        }) = events.last()
        else {
            bail!("No event sent");
        };

        assert_eq!(
            doc.extract_bytes(),
            b"I am a special document",
            "document did not match"
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
