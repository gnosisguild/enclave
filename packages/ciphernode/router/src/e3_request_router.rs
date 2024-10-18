use crate::CommitteeMetaFeature;
use crate::E3RequestContext;
use crate::E3RequestContextParams;
use crate::E3RequestContextSnapshot;
use crate::Repositories;
use actix::{Actor, Addr, Context, Handler};
use anyhow::*;
use async_trait::async_trait;
use data::Checkpoint;
use data::DataStore;
use data::FromSnapshotWithParams;
use data::Repository;
use data::Snapshot;
use enclave_core::E3RequestComplete;
use enclave_core::{E3id, EnclaveEvent, EventBus, Subscribe};
use serde::Deserialize;
use serde::Serialize;
use std::collections::HashSet;
use std::{collections::HashMap, sync::Arc};

/// Helper class to buffer events for downstream instances incase events arrive in the wrong order
#[derive(Default)]
pub struct EventBuffer {
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
/// Format of the hook that needs to be passed to E3RequestRouter
// pub type EventHook = Box<dyn FnMut(&mut E3RequestContext, EnclaveEvent)>;
// pub type Hydrator = Box<dyn FnMut(&mut E3RequestContext, &E3RequestContextSnapshot)>;
// pub type E3Feature = (EventHook, Hydrator);

#[async_trait]
pub trait E3Feature: Send + Sync + 'static {
    fn on_event(&self, ctx: &mut E3RequestContext, evt: &EnclaveEvent);
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
    store: Repository<E3RequestRouterSnapshot>,
}

pub struct E3RequestRouterParams {
    features: Arc<Vec<Box<dyn E3Feature>>>,
    bus: Addr<EventBus>,
    store: Repository<E3RequestRouterSnapshot>,
}

impl E3RequestRouter {
    pub fn builder(bus: Addr<EventBus>, store: DataStore) -> E3RequestRouterBuilder {
        let repositories: Repositories = store.into();
        let builder = E3RequestRouterBuilder {
            bus,
            features: vec![],
            store: repositories.router(),
        };

        // Everything needs the committe meta factory so adding it here by default
        builder.add_feature(CommitteeMetaFeature::create())
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

        let repositories: Repositories = self.repository().into();
        let context = self.contexts.entry(e3_id.clone()).or_insert_with(|| {
            E3RequestContext::from_params(E3RequestContextParams {
                e3_id: e3_id.clone(),
                store: repositories.context(&e3_id),
                features: self.features.clone(),
            })
        });

        for feature in self.features.clone().iter() {
            feature.on_event(context, &msg);
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
    fn repository(&self) -> Repository<E3RequestRouterSnapshot> {
        self.store.clone()
    }
}

#[async_trait]
impl FromSnapshotWithParams for E3RequestRouter {
    type Params = E3RequestRouterParams;

    async fn from_snapshot(params: Self::Params, snapshot: Self::Snapshot) -> Result<Self> {
        let mut contexts = HashMap::new();

        let repositories: Repositories = params.store.into();
        for e3_id in snapshot.contexts {
            let Some(ctx_snapshot) = repositories.context(&e3_id).read().await? else {
                continue;
            };

            contexts.insert(
                e3_id.clone(),
                E3RequestContext::from_snapshot(
                    E3RequestContextParams {
                        store: repositories.context(&e3_id),
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
            store: repositories.router(),
        })
    }
}

/// Builder for E3RequestRouter
pub struct E3RequestRouterBuilder {
    pub bus: Addr<EventBus>,
    pub features: Vec<Box<dyn E3Feature>>,
    pub store: Repository<E3RequestRouterSnapshot>,
}

impl E3RequestRouterBuilder {
    pub fn add_feature(mut self, listener: Box<dyn E3Feature>) -> Self {
        self.features.push(listener);
        self
    }

    pub async fn build(self) -> Result<Addr<E3RequestRouter>> {
        let repositories: Repositories = self.store.into();
        let router_repo = repositories.router();
        let snapshot: Option<E3RequestRouterSnapshot> = router_repo.read().await?;
        let params = E3RequestRouterParams {
            features: self.features.into(),
            bus: self.bus.clone(),

            store: router_repo,
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
