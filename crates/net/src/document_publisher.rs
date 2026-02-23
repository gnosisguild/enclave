// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::{
    events::{
        call_and_await_response, DocumentPublishedNotification, GossipData, NetCommand, NetEvent,
    },
    ContentHash,
};
use actix::prelude::*;
use anyhow::Context;
use anyhow::Result;
use chrono::{DateTime, Utc};
use e3_events::{
    prelude::*, trap, trap_fut, BusHandle, CiphernodeSelected, CorrelationId, DocumentKind,
    DocumentMeta, DocumentReceived, E3RequestComplete, E3id, EType, EnclaveEvent, EnclaveEventData,
    EncryptionKeyCreated, EncryptionKeyReceived, Event, EventContext, EventSource, EventType,
    Filter, PartyId, PublishDocumentRequested, Sequenced, ThresholdShareCreated, TypedEvent,
};
use e3_utils::ArcBytes;
use e3_utils::NotifySync;
use e3_utils::{
    retry::{retry_with_backoff, to_retry},
    MAILBOX_LIMIT,
};
use futures::TryFutureExt;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::{broadcast, mpsc};
use tracing::{debug, info};

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
    bus: BusHandle,
    /// NetCommand sender to forward commands to the NetInterface
    tx: mpsc::Sender<NetCommand>,
    /// NetEvent receiver to resubscribe for events from the NetInterface. This is in an Arc so
    /// that we do not do excessive resubscribes without actually listening for events.
    rx: Arc<broadcast::Receiver<NetEvent>>,
    /// The gossipsub broadcast topic
    topic: String,
    /// Set of E3ids we are interested in
    ids: HashMap<E3id, PartyId>,
    /// Track DHT content hashes per E3 for cleanup on completion
    dht_keys: HashMap<E3id, Vec<ContentHash>>,
}

impl DocumentPublisher {
    /// Create a new DocumentPublisher actor
    pub fn new(
        bus: &BusHandle,
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
            dht_keys: HashMap::new(),
        }
    }

    /// This is needed to create simulation libp2p event routers
    pub fn is_document_publisher_event(event: &EnclaveEvent) -> bool {
        // Add a list of events with paylods for the DHT
        match event.get_data() {
            EnclaveEventData::PublishDocumentRequested(_) => true,
            EnclaveEventData::ThresholdShareCreated(_) => true,
            EnclaveEventData::EncryptionKeyCreated(_) => true,
            _ => false,
        }
    }

    /// Setup the DocumentPublisher and start listening for GossipEvents
    pub fn setup(
        bus: &BusHandle,
        tx: &mpsc::Sender<NetCommand>,
        rx: &Arc<broadcast::Receiver<NetEvent>>,
        topic: impl Into<String>,
    ) -> Addr<Self> {
        let mut events = rx.resubscribe();
        let addr = Self::new(bus, tx, rx, topic).start();
        EventConverter::setup(bus);
        // Listen on all events
        bus.subscribe(EventType::All, addr.clone().recipient());

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
        if let Some(keys) = self.dht_keys.remove(&event.e3_id) {
            if !keys.is_empty() {
                info!(
                    "Pruning {} DHT records for completed E3 {}",
                    keys.len(),
                    event.e3_id
                );
                let _ = self.tx.try_send(NetCommand::DhtRemoveRecords { keys });
            }
        }
        Ok(())
    }
}

impl Actor for DocumentPublisher {
    type Context = actix::Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.set_mailbox_capacity(MAILBOX_LIMIT)
    }
}

impl Handler<EnclaveEvent> for DocumentPublisher {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        let (msg, ec) = msg.into_components();
        match msg {
            EnclaveEventData::PublishDocumentRequested(data) => {
                ctx.notify(TypedEvent::new(data, ec))
            }
            EnclaveEventData::CiphernodeSelected(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::E3RequestComplete(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            _ => (),
        }
    }
}

impl Handler<TypedEvent<PublishDocumentRequested>> for DocumentPublisher {
    type Result = ResponseFuture<()>;
    fn handle(
        &mut self,
        msg: TypedEvent<PublishDocumentRequested>,
        _: &mut Self::Context,
    ) -> Self::Result {
        let tx = self.tx.clone();
        let (msg, ec) = msg.into_components();

        let key = ContentHash::from_content(&msg.value);
        self.dht_keys
            .entry(msg.meta.e3_id.clone())
            .or_default()
            .push(key);

        let rx = self.rx.clone();
        let bus = self.bus.clone();
        let topic = self.topic.clone();
        trap_fut(
            EType::IO,
            &bus.with_ec(&ec),
            handle_publish_document_requested(tx, rx, msg, topic, bus),
        )
    }
}

impl Handler<TypedEvent<CiphernodeSelected>> for DocumentPublisher {
    type Result = ();
    fn handle(
        &mut self,
        msg: TypedEvent<CiphernodeSelected>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let (msg, ec) = msg.into_components();
        trap(EType::DocumentPublishing, &self.bus.with_ec(&ec), || {
            self.handle_ciphernode_selected(msg)
        })
    }
}

impl Handler<TypedEvent<E3RequestComplete>> for DocumentPublisher {
    type Result = ();
    fn handle(
        &mut self,
        msg: TypedEvent<E3RequestComplete>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let (msg, ec) = msg.into_components();
        trap(EType::DocumentPublishing, &self.bus.with_ec(&ec), || {
            self.handle_e3_request_complete(msg)
        })
    }
}

/// Receiving DocumentPublishedNotification from libp2p
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
        let rx = self.rx.clone();
        let msg = msg.clone();
        trap_fut(
            EType::IO,
            &bus,
            handle_document_published_notification(tx, rx, bus.clone(), ids, msg),
        )
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
    bus: BusHandle,
) -> Result<()> {
    let value = event.value;
    let key = ContentHash::from_content(&value);
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
    let notification = DocumentPublishedNotification::new(event.meta, key, bus.ts()?);
    broadcast_document_published_notification(tx, rx, notification, topic).await?;
    Ok(())
}

/// Called when we receive a notification from the net_interface
pub async fn handle_document_published_notification(
    net_cmds: mpsc::Sender<NetCommand>,
    net_events: Arc<broadcast::Receiver<NetEvent>>,
    bus: BusHandle,
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
    bus.publish_from_remote(
        DocumentReceived {
            meta: event.meta,
            value,
        },
        event.ts,
        None,
        EventSource::Net,
    )?;

    Ok(())
}

/// Call DhtPutRecord Command on the NetInterface and handle the results
async fn put_record(
    net_cmds: mpsc::Sender<NetCommand>,
    net_events: Arc<broadcast::Receiver<NetEvent>>,
    expires: Option<Instant>,
    value: ArcBytes,
    key: ContentHash,
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
    key: ContentHash,
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

/// Converts between internal events and network documents.
///
/// Responsibilities:
/// - Outgoing: Converts ThresholdShareCreated → party-filtered PublishDocumentRequested
/// - Incoming: Converts DocumentReceived → ThresholdShareCreated/EncryptionKeyCreated
///
/// Note: Party filtering is done by DocumentPublisher BEFORE fetching from DHT.
pub struct EventConverter {
    bus: BusHandle,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
enum ReceivableDocument {
    ThresholdShareCreated(ThresholdShareCreated),
    EncryptionKeyCreated(EncryptionKeyCreated),
}

impl ReceivableDocument {
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
}

impl EventConverter {
    pub fn new(bus: &BusHandle) -> Self {
        Self { bus: bus.clone() }
    }

    pub fn setup(bus: &BusHandle) -> Addr<Self> {
        let addr = Self::new(bus).start();
        bus.subscribe(EventType::ThresholdShareCreated, addr.clone().into());
        bus.subscribe(EventType::EncryptionKeyCreated, addr.clone().into());
        bus.subscribe(EventType::DocumentReceived, addr.clone().into());
        addr
    }

    /// Publish a receivable document with party filter
    fn publish_filtered(
        &self,
        receivable: ReceivableDocument,
        e3_id: &E3id,
        party_id: u64,
        ctx: EventContext<Sequenced>,
    ) -> Result<()> {
        let value = ArcBytes::from_bytes(&receivable.to_bytes()?);
        let meta = DocumentMeta::new(
            e3_id.clone(),
            DocumentKind::TrBFV,
            vec![Filter::Item(party_id)],
            None,
        );
        self.bus
            .publish(PublishDocumentRequested::new(meta, value), ctx)?;
        Ok(())
    }

    /// Local node created a threshold share (already split per-party by ThresholdKeyshare).
    /// Publishes the single-party document with appropriate filter.
    pub fn handle_threshold_share_created(
        &self,
        msg: TypedEvent<ThresholdShareCreated>,
    ) -> Result<()> {
        let (msg, ctx) = msg.into_components();
        if msg.external {
            return Ok(());
        }
        let target_party_id = msg.target_party_id;

        info!(
            "Publishing ThresholdShare from party {} for target party {} (E3 {})",
            msg.share.party_id, target_party_id, msg.e3_id
        );

        let e3_id = msg.e3_id.clone();

        self.publish_filtered(
            ReceivableDocument::ThresholdShareCreated(msg),
            &e3_id,
            target_party_id,
            ctx,
        )?;

        Ok(())
    }
    fn handle_encryption_key_created(&self, msg: TypedEvent<EncryptionKeyCreated>) -> Result<()> {
        let (msg, ctx) = msg.into_components();
        if msg.external {
            return Ok(());
        }

        let meta = DocumentMeta::new(msg.e3_id.clone(), DocumentKind::TrBFV, vec![], None);
        let receivable = ReceivableDocument::EncryptionKeyCreated(msg);
        let value = ArcBytes::from_bytes(&receivable.to_bytes()?);
        self.bus
            .publish(PublishDocumentRequested::new(meta, value), ctx)?;
        Ok(())
    }

    /// Convert received document to internal events.
    /// Note: Filtering already happened in DocumentPublisher before DHT fetch.
    fn handle_document_received(&self, msg: TypedEvent<DocumentReceived>) -> Result<()> {
        let (msg, ctx) = msg.into_components();
        let receivable = ReceivableDocument::from_bytes(&msg.value.extract_bytes())
            .context("Could not deserialize document bytes")?;
        match receivable {
            ReceivableDocument::ThresholdShareCreated(evt) => {
                debug!(
                    "Received ThresholdShareCreated from party {} for target party {}",
                    evt.share.party_id, evt.target_party_id
                );
                self.bus.publish(
                    ThresholdShareCreated {
                        external: true,
                        e3_id: evt.e3_id,
                        share: evt.share,
                        target_party_id: evt.target_party_id,
                    },
                    ctx.clone(),
                )?;
            }
            ReceivableDocument::EncryptionKeyCreated(evt) => {
                debug!(
                    "Received EncryptionKeyCreated from party {}",
                    evt.key.party_id
                );
                self.bus.publish(
                    EncryptionKeyReceived {
                        e3_id: evt.e3_id,
                        key: evt.key,
                    },
                    ctx,
                )?;
            }
        }
        Ok(())
    }
}

impl Actor for EventConverter {
    type Context = actix::Context<Self>;
}

impl Handler<EnclaveEvent> for EventConverter {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        let (data, ec) = msg.into_components();
        match data {
            EnclaveEventData::ThresholdShareCreated(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::EncryptionKeyCreated(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::DocumentReceived(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            _ => (),
        }
    }
}

impl Handler<TypedEvent<ThresholdShareCreated>> for EventConverter {
    type Result = ();
    fn handle(
        &mut self,
        msg: TypedEvent<ThresholdShareCreated>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        trap(
            EType::DocumentPublishing,
            &self.bus.with_ec(msg.get_ctx()),
            || self.handle_threshold_share_created(msg),
        )
    }
}

impl Handler<TypedEvent<EncryptionKeyCreated>> for EventConverter {
    type Result = ();
    fn handle(
        &mut self,
        msg: TypedEvent<EncryptionKeyCreated>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        trap(
            EType::DocumentPublishing,
            &self.bus.with_ec(msg.get_ctx()),
            || self.handle_encryption_key_created(msg),
        )
    }
}

impl Handler<TypedEvent<DocumentReceived>> for EventConverter {
    type Result = ();
    fn handle(
        &mut self,
        msg: TypedEvent<DocumentReceived>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        trap(
            EType::DocumentPublishing,
            &self.bus.with_ec(msg.get_ctx()),
            || self.handle_document_received(msg),
        )
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, num::NonZero, time::Duration};

    use super::*;
    use crate::events::NetCommand;
    use actix::Addr;
    use anyhow::{bail, Result};
    use e3_ciphernode_builder::EventSystem;
    use e3_events::{
        BusHandle, CiphernodeSelected, DocumentKind, DocumentMeta, E3id, EnclaveError,
        EnclaveEvent, GetEvents, HistoryCollector, PublishDocumentRequested, TakeEvents,
    };
    use libp2p::kad::{GetRecordError, PutRecordError, RecordKey};
    use tokio::{
        sync::{broadcast, mpsc},
        time::{sleep, timeout},
    };
    use tracing::subscriber::DefaultGuard;

    fn setup_test() -> Result<(
        DefaultGuard,
        BusHandle,
        mpsc::Sender<NetCommand>,
        mpsc::Receiver<NetCommand>,
        broadcast::Sender<NetEvent>,
        Arc<broadcast::Receiver<NetEvent>>,
        Addr<HistoryCollector<EnclaveEvent>>,
        Addr<HistoryCollector<EnclaveEvent>>,
        Addr<DocumentPublisher>,
    )> {
        use tracing_subscriber::{fmt, EnvFilter};

        let subscriber = fmt()
            .with_env_filter(EnvFilter::new("debug"))
            .with_test_writer()
            .finish();

        let guard = tracing::subscriber::set_default(subscriber);

        let system = EventSystem::new().with_fresh_bus();
        let bus = system.handle()?.enable("test");
        let (net_cmd_tx, net_cmd_rx) = mpsc::channel(100);
        let (net_evt_tx, net_evt_rx) = broadcast::channel(100);
        let net_evt_rx = Arc::new(net_evt_rx);
        let history = HistoryCollector::<EnclaveEvent>::new().start();
        let error = HistoryCollector::<EnclaveEvent>::new().start();
        bus.subscribe(EventType::All, history.clone().recipient());
        bus.subscribe(EventType::EnclaveError, error.clone().recipient());
        let publisher = DocumentPublisher::setup(&bus, &net_cmd_tx, &net_evt_rx, "topic");

        Ok((
            guard, bus, net_cmd_tx, net_cmd_rx, net_evt_tx, net_evt_rx, history, error, publisher,
        ))
    }

    #[actix::test]
    async fn test_publishes_document() -> Result<()> {
        let (_guard, bus, _net_cmd_tx, mut net_cmd_rx, net_evt_tx, _net_evt_rx, _, _, _) =
            setup_test()?;
        let value = ArcBytes::from_bytes(b"I am a special document");
        let expires_at = Some(Utc::now() + chrono::Duration::days(1));
        let e3_id = E3id::new("1243", 1);

        // 1. Send a request to publish
        bus.publish_without_context(PublishDocumentRequested {
            meta: DocumentMeta::new(e3_id, DocumentKind::TrBFV, vec![], expires_at),
            value: value.clone(),
        })?;

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
        let mut mykad: HashMap<ContentHash, Vec<u8>> = HashMap::new();
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
            setup_test()?;

        let value = b"I am a special document".to_vec();
        let expires_at = Some(Utc::now() + chrono::Duration::days(1));
        let e3_id = E3id::new("1243", 1);
        let cid = ContentHash::from_content(&value);

        // 1. Ensure the publisher is interested in the id by receiving CiphernodeSelected
        bus.publish_without_context(CiphernodeSelected {
            e3_id: e3_id.clone(),
            threshold_m: 3,
            threshold_n: 5,
            ..CiphernodeSelected::default()
        })?;

        net_evt_tx.send(NetEvent::GossipData(
            GossipData::DocumentPublishedNotification(DocumentPublishedNotification {
                key: cid.clone(),
                meta: DocumentMeta::new(e3_id, DocumentKind::TrBFV, vec![], expires_at),
                ts: 123,
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
        ) = setup_test()?;
        let value = ArcBytes::from_bytes(b"I am a special document");
        let expires_at = Some(Utc::now() + chrono::Duration::days(1));
        let e3_id = E3id::new("1243", 1);

        // Send a request to publish
        bus.publish_without_context(PublishDocumentRequested {
            meta: DocumentMeta::new(e3_id, DocumentKind::TrBFV, vec![], expires_at),
            value: value.clone(),
        })?;

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
            setup_test()?;

        let value = ArcBytes::from_bytes(b"I am a special document");
        let expires_at = Utc::now() + chrono::Duration::days(1);
        let e3_id = E3id::new("1243", 1);
        let cid = ContentHash::from_content(&value);

        // 1. Ensure the publisher is interested in the id by receiving CiphernodeSelected
        bus.publish_without_context(CiphernodeSelected {
            e3_id: e3_id.clone(),
            threshold_m: 3,
            threshold_n: 5,
            ..CiphernodeSelected::default()
        })?;

        // 2. Dispatch a NetEvent from the NetInterface signaling that a document was published
        net_evt_tx.send(NetEvent::GossipData(
            GossipData::DocumentPublishedNotification(DocumentPublishedNotification {
                key: ContentHash::from_content(&b"wrong document".to_vec()),
                meta: DocumentMeta::new(
                    E3id::new("1111", 1),
                    DocumentKind::TrBFV,
                    vec![],
                    Some(expires_at),
                ),
                ts: 123,
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
                ts: 100,
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
            value, // This will error because this is not a ReceivableDocument which is fine
        })?;

        // wait for events to settle
        sleep(Duration::from_millis(100)).await;

        // Check event was dispatched
        let events = history.send(GetEvents::new()).await?;
        let Some(EnclaveEventData::DocumentReceived(DocumentReceived { value: doc, .. })) = events
            .iter()
            // Filter out the error
            .filter(|e| e.event_type() != "EnclaveError")
            .last()
            .map(|e| e.get_data())
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
