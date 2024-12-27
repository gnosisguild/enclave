use std::{collections::HashMap, sync::Arc};

use crate::{E3Feature, EventBuffer, HetrogenousMap, TypedKey};
use actix::Recipient;
use anyhow::Result;
use async_trait::async_trait;
use data::{
    Checkpoint, FromSnapshotWithParams, Repositories, RepositoriesFactory, Repository, Snapshot,
};
use enclave_core::{E3id, EnclaveEvent};
use serde::{Deserialize, Serialize};

fn init_recipients() -> HashMap<String, Option<Recipient<EnclaveEvent>>> {
    HashMap::from([
        ("keyshare".to_owned(), None),
        ("plaintext".to_owned(), None),
        ("publickey".to_owned(), None),
    ])
}

/// Context that is set to each event hook. Hooks can use this context to gather dependencies if
/// they need to instantiate struct instances or actors.
// TODO: remove Addr<T> imports as we need to be able to move the features out of the hooks file
// without circular deps
// TODO: remove Arc<Fhe> import as we need to be able to move the Fhe feature out of the hooks
// file without circular deps
pub struct E3RequestContext {
    pub e3_id: E3id,
    pub recipients: HashMap<String, Option<Recipient<EnclaveEvent>>>, // NOTE: can be a None value
    pub dependencies: HetrogenousMap,
    pub store: Repository<E3RequestContextSnapshot>,
}

#[derive(Serialize, Deserialize)]
pub struct E3RequestContextSnapshot {
    pub e3_id: E3id,
    pub recipients: Vec<String>,
    pub dependencies: Vec<String>,
}

impl E3RequestContextSnapshot {
    pub fn contains(&self, key: &str) -> bool {
        self.recipients.contains(&key.to_string()) || self.dependencies.contains(&key.to_string())
    }
}

pub struct E3RequestContextParams {
    pub store: Repository<E3RequestContextSnapshot>,
    pub e3_id: E3id,
    pub features: Arc<Vec<Box<dyn E3Feature>>>,
}

impl E3RequestContext {
    pub fn from_params(params: E3RequestContextParams) -> Self {
        Self {
            e3_id: params.e3_id,
            store: params.store,
            recipients: init_recipients(),
            dependencies: HetrogenousMap::new(),
        }
    }

    fn recipients(&self) -> Vec<(String, Option<Recipient<EnclaveEvent>>)> {
        self.recipients
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    pub fn forward_message(&self, msg: &EnclaveEvent, buffer: &mut EventBuffer) {
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

    pub fn forward_message_now(&self, msg: &EnclaveEvent) {
        self.recipients().into_iter().for_each(|(_, recipient)| {
            if let Some(act) = recipient {
                act.do_send(msg.clone());
            }
        });
    }

    pub fn set_event_recipient(
        &mut self,
        key: impl Into<String>,
        value: Option<Recipient<EnclaveEvent>>,
    ) {
        self.recipients.insert(key.into(), value);
        self.checkpoint();
    }

    pub fn get_event_recipient(&self, key: impl Into<String>) -> Option<&Recipient<EnclaveEvent>> {
        self.recipients
            .get(&key.into())
            .and_then(|opt| opt.as_ref())
    }

    pub fn set_dependency<T>(&mut self, key: TypedKey<T>, value: T)
    where
        T: Send + Sync + 'static,
    {
        self.dependencies.insert(key, value);
        self.checkpoint();
    }

    pub fn get_dependency<T>(&self, key: TypedKey<T>) -> Option<&T>
    where
        T: Send + Sync + 'static,
    {
        self.dependencies.get(key)
    }
}

impl RepositoriesFactory for E3RequestContext {
    fn repositories(&self) -> Repositories {
        self.repository().clone().into()
    }
}

#[async_trait]
impl Snapshot for E3RequestContext {
    type Snapshot = E3RequestContextSnapshot;

    fn snapshot(&self) -> Result<Self::Snapshot> {
        Ok(Self::Snapshot {
            e3_id: self.e3_id.clone(),
            dependencies: self.dependencies.keys(),
            recipients: self.recipients.keys().cloned().collect(),
        })
    }
}

#[async_trait]
impl FromSnapshotWithParams for E3RequestContext {
    type Params = E3RequestContextParams;
    async fn from_snapshot(params: Self::Params, snapshot: Self::Snapshot) -> Result<Self> {
        let mut ctx = Self {
            e3_id: params.e3_id,
            store: params.store,
            recipients: init_recipients(),
            dependencies: HetrogenousMap::new(),
        };

        for feature in params.features.iter() {
            feature.hydrate(&mut ctx, &snapshot).await?;
        }

        Ok(ctx)
    }
}

impl Checkpoint for E3RequestContext {
    fn repository(&self) -> &Repository<E3RequestContextSnapshot> {
        &self.store
    }
}
