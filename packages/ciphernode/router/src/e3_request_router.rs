use crate::CommitteMetaFeature;

use super::CommitteeMeta;
use actix::{Actor, Addr, Context, Handler, Recipient};
use aggregator::PlaintextAggregator;
use aggregator::PublicKeyAggregator;
use anyhow::*;
use async_trait::async_trait;
use data::Checkpoint;
use data::DataStore;
use data::FromSnapshotWithParams;
use data::Snapshot;
use data::WithPrefix;
use enclave_core::E3RequestComplete;
use enclave_core::{E3id, EnclaveEvent, EventBus, Subscribe};
use fhe::Fhe;
use keyshare::Keyshare;
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashSet;
use std::pin::Pin;
use std::{collections::HashMap, sync::Arc};

/// Helper class to buffer events for downstream instances incase events arrive in the wrong order
#[derive(Default)]
struct EventBuffer {
    buffer: HashMap<String, Vec<EnclaveEvent>>,
}

impl EventBuffer {
    pub fn add(&mut self, key: &str, event: EnclaveEvent) {
        self.buffer.entry(key.to_string()).or_default().push(event)
    }

    pub fn take(&mut self, key: &str) -> Vec<EnclaveEvent> {
        self.buffer
            .get_mut(key)
            .map(std::mem::take)
            .unwrap_or_default()
    }
}

/// Context that is set to each event hook. Hooks can use this context to gather dependencies if
/// they need to instantiate struct instances or actors.
pub struct E3RequestContext {
    pub e3_id: E3id,
    pub keyshare: Option<Addr<Keyshare>>,
    pub fhe: Option<Arc<Fhe>>,
    pub plaintext: Option<Addr<PlaintextAggregator>>,
    pub publickey: Option<Addr<PublicKeyAggregator>>,
    pub meta: Option<CommitteeMeta>,
    pub store: DataStore,
}

#[derive(Serialize, Deserialize)]
pub struct E3RequestContextSnapshot {
    pub keyshare: Option<String>,
    pub fhe: Option<String>,
    pub plaintext: Option<String>,
    pub publickey: Option<String>,
    pub meta: Option<String>,
}

pub struct E3RequestContextParams {
    pub store: DataStore,
    pub e3_id: E3id,
    pub features: Arc<Vec<Box<dyn E3Feature>>>,
}

impl E3RequestContext {
    pub fn from_params(params: E3RequestContextParams) -> Self {
        Self {
            e3_id: params.e3_id,
            store: params.store,
            fhe: None,
            keyshare: None,
            meta: None,
            plaintext: None,
            publickey: None,
        }
    }

    fn recipients(&self) -> Vec<(String, Option<Recipient<EnclaveEvent>>)> {
        vec![
            (
                "keyshare".to_owned(),
                self.keyshare.clone().map(|addr| addr.into()),
            ),
            (
                "plaintext".to_owned(),
                self.plaintext.clone().map(|addr| addr.into()),
            ),
            (
                "publickey".to_owned(),
                self.publickey.clone().map(|addr| addr.into()),
            ),
        ]
    }

    fn forward_message(&self, msg: &EnclaveEvent, buffer: &mut EventBuffer) {
        self.recipients().into_iter().for_each(|(key, recipient)| {
            if let Some(act) = recipient {
                act.do_send(msg.clone());
                for m in buffer.take(&key) {
                    act.do_send(m);
                }
            } else {
                buffer.add(&key, msg.clone());
            }
        });
    }

    /// Accept a DataStore ID and a Keystore actor address
    pub fn set_keyshare(&mut self, value: Addr<Keyshare>) {
        self.keyshare = Some(value);
        self.checkpoint();
    }

    /// Accept a DataStore ID and a Keystore actor address
    pub fn set_plaintext(&mut self, value: Addr<PlaintextAggregator>) {
        self.plaintext = Some(value);
        self.checkpoint();
    }

    /// Accept a DataStore ID and a Keystore actor address
    pub fn set_publickey(&mut self, value: Addr<PublicKeyAggregator>) {
        self.publickey = Some(value);
        self.checkpoint();
    }

    /// Accept a DataStore ID and an Arc instance of the Fhe wrapper
    pub fn set_fhe(&mut self, value: Arc<Fhe>) {
        self.fhe = Some(value.clone());
        self.checkpoint();
    }

    /// Accept a Datastore ID and a metadata object
    pub fn set_meta(&mut self, value: CommitteeMeta) {
        self.meta = Some(value.clone());
        self.checkpoint();
    }

    pub fn get_keyshare(&self) -> Option<&Addr<Keyshare>> {
        self.keyshare.as_ref()
    }

    pub fn get_plaintext(&self) -> Option<&Addr<PlaintextAggregator>> {
        self.plaintext.as_ref()
    }

    pub fn get_publickey(&self) -> Option<&Addr<PublicKeyAggregator>> {
        self.publickey.as_ref()
    }

    pub fn get_fhe(&self) -> Option<&Arc<Fhe>> {
        self.fhe.as_ref()
    }

    pub fn get_meta(&self) -> Option<&CommitteeMeta> {
        self.meta.as_ref()
    }

    pub fn get_store(&self) -> DataStore {
        self.store.clone()
    }
}

#[async_trait]
impl Snapshot for E3RequestContext {
    type Snapshot = E3RequestContextSnapshot;

    fn snapshot(&self) -> Self::Snapshot {
        let e3_id = self.e3_id.clone();
        let meta = self.meta.as_ref().map(|_| format!("//meta/{e3_id}"));
        let fhe = self.fhe.as_ref().map(|_| format!("//fhe/{e3_id}"));
        let publickey = self
            .publickey
            .as_ref()
            .map(|_| format!("//publickey/{e3_id}"));
        let plaintext = self
            .plaintext
            .as_ref()
            .map(|_| format!("//plaintext/{e3_id}"));
        let keyshare = self
            .keyshare
            .as_ref()
            .map(|_| format!("//keyshare/{e3_id}"));

        Self::Snapshot {
            meta,
            fhe,
            publickey,
            plaintext,
            keyshare,
        }
    }
}

#[async_trait]
impl FromSnapshotWithParams for E3RequestContext {
    type Params = E3RequestContextParams;
    async fn from_snapshot(params: Self::Params, snapshot: Self::Snapshot) -> Result<Self> {
        let mut ctx = Self {
            e3_id: params.e3_id,
            store: params.store,
            fhe: None,
            keyshare: None,
            meta: None,
            plaintext: None,
            publickey: None,
        };

        for feature in params.features.iter() {
            feature.hydrate(&mut ctx, &snapshot).await?;
        }

        Ok(ctx)
    }
}

impl Checkpoint for E3RequestContext {
    fn get_store(&self) -> DataStore {
        self.store.clone()
    }
}

/// Format of the hook that needs to be passed to E3RequestRouter
// pub type EventHook = Box<dyn FnMut(&mut E3RequestContext, EnclaveEvent)>;
// pub type Hydrator = Box<dyn FnMut(&mut E3RequestContext, &E3RequestContextSnapshot)>;
// pub type E3Feature = (EventHook, Hydrator);

#[async_trait]
pub trait E3Feature: Send + Sync + 'static {
    fn event(&self, ctx: &mut E3RequestContext, evt: &EnclaveEvent);
    async fn hydrate(
        &self,
        ctx: &mut E3RequestContext,
        snapshot: &E3RequestContextSnapshot,
    ) -> Result<()>;
}

/// E3RequestRouter will register features that receive an E3_id specific context. After features
/// have run e3_id specific messages are forwarded to all instances on the context. This enables
/// features to lazily register instances that have the correct dependencies available per e3_id
/// request
// TODO: setup typestate pattern so that we have to place features within correct order of
// dependencies
pub struct E3RequestRouter {
    contexts: HashMap<E3id, E3RequestContext>,
    completed: HashSet<E3id>,
    features: Arc<Vec<Box<dyn E3Feature>>>,
    buffer: EventBuffer,
    bus: Addr<EventBus>,
    store: DataStore,
}

pub struct E3RequestRouterParams {
    features: Arc<Vec<Box<dyn E3Feature>>>,
    bus: Addr<EventBus>,
    store: DataStore,
}

impl E3RequestRouter {
    pub fn builder(bus: Addr<EventBus>, store: DataStore) -> E3RequestRouterBuilder {
        let builder = E3RequestRouterBuilder {
            bus,
            features: vec![],
            store,
        };

        // Everything needs the committe meta factory so adding it here by default
        builder.add_feature(CommitteMetaFeature::create())
    }

    pub fn from_params(params: E3RequestRouterParams) -> Self {
        Self {
            features: params.features,
            bus: params.bus.clone(),
            store: params.store.clone(),
            completed: HashSet::new(),
            contexts: HashMap::new(),
            buffer: EventBuffer {
                buffer: HashMap::new(),
            },
        }
    }
}

impl Actor for E3RequestRouter {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for E3RequestRouter {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, _: &mut Self::Context) -> Self::Result {
        let Some(e3_id) = msg.get_e3_id() else {
            return;
        };

        if self.completed.contains(&e3_id) {
            // TODO: Log warning that e3 event was received for completed e3_id
            return;
        }

        let context = self.contexts.entry(e3_id.clone()).or_insert_with(|| {
            E3RequestContext::from_params(E3RequestContextParams {
                e3_id: e3_id.clone(),
                store: self.store.at(&format!("//context/{e3_id}")),
                features: self.features.clone(),
            })
        });

        for feature in self.features.clone().iter() {
            feature.event(context, &msg);
        }

        context.forward_message(&msg, &mut self.buffer);

        match &msg {
            EnclaveEvent::PlaintextAggregated { .. } => {
                // Here we are detemining that by receiving the PlaintextAggregated event our request is
                // complete and we can notify everyone. This might change as we consider other factors
                // when determining if the request is complete
                let event = EnclaveEvent::from(E3RequestComplete {
                    e3_id: e3_id.clone(),
                });

                // Send to bus so all other actors can react to a request being complete.
                self.bus.do_send(event);
            }
            EnclaveEvent::E3RequestComplete { .. } => {
                // Note this will be sent above to the children who can kill themselves based on
                // the event
                self.contexts.remove(&e3_id);
                self.completed.insert(e3_id);
            }
            _ => (),
        }

        self.checkpoint();
    }
}

#[derive(Serialize, Deserialize)]
pub struct E3RequestRouterSnapshot {
    contexts: Vec<E3id>,
    completed: HashSet<E3id>,
}

impl Snapshot for E3RequestRouter {
    type Snapshot = E3RequestRouterSnapshot;
    fn snapshot(&self) -> Self::Snapshot {
        let contexts = self.contexts.keys().cloned().collect();
        let completed = self.completed.clone();

        Self::Snapshot {
            completed,
            contexts,
        }
    }
}

impl Checkpoint for E3RequestRouter {
    fn get_store(&self) -> DataStore {
        self.store.clone()
    }
}

#[async_trait]
impl FromSnapshotWithParams for E3RequestRouter {
    type Params = E3RequestRouterParams;

    async fn from_snapshot(params: Self::Params, snapshot: Self::Snapshot) -> Result<Self> {
        let mut contexts = HashMap::new();
        for e3_id in snapshot.contexts {
            let Some(ctx_snapshot) = params.store.read(format!("//context/{e3_id}")).await? else {
                continue;
            };

            contexts.insert(
                e3_id.clone(),
                E3RequestContext::from_snapshot(
                    E3RequestContextParams {
                        store: params.store.clone(),
                        e3_id: e3_id.clone(),
                        features: params.features.clone(),
                    },
                    ctx_snapshot,
                )
                .await?,
            );
        }

        Ok(E3RequestRouter {
            contexts,
            completed: snapshot.completed,
            features: params.features.into(),
            buffer: EventBuffer::default(),
            bus: params.bus,
            store: params.store,
        })
    }
}

/// Builder for E3RequestRouter
pub struct E3RequestRouterBuilder {
    pub bus: Addr<EventBus>,
    pub features: Vec<Box<dyn E3Feature>>,
    pub store: DataStore,
}

impl E3RequestRouterBuilder {
    pub fn add_feature(mut self, listener: Box<dyn E3Feature>) -> Self {
        self.features.push(listener);
        self
    }

    pub async fn build(self) -> Result<Addr<E3RequestRouter>> {
        let snapshot: Option<E3RequestRouterSnapshot> = self.store.at("").read("//router").await?;
        let params = E3RequestRouterParams {
            features: self.features.into(),
            bus: self.bus.clone(),

            store: self.store,
        };

        let e3r = match snapshot {
            Some(snapshot) => E3RequestRouter::from_snapshot(params, snapshot).await?,
            None => E3RequestRouter::from_params(params),
        };

        let addr = e3r.start();
        self.bus
            .do_send(Subscribe::new("*", addr.clone().recipient()));
        Ok(addr)
    }
}
