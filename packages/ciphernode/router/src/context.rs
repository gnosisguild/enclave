use std::sync::Arc;

use crate::{CommitteeMeta, E3Feature, EventBuffer};
use actix::{Addr, Recipient};
use aggregator::{PlaintextAggregator, PublicKeyAggregator};
use anyhow::Result;
use async_trait::async_trait;
use data::{
    Checkpoint, FromSnapshotWithParams, Repositories, RepositoriesFactory, Repository, Snapshot,
};
use enclave_core::{E3id, EnclaveEvent};
use fhe::Fhe;
use keyshare::Keyshare;
use serde::{Deserialize, Serialize};

/// Context that is set to each event hook. Hooks can use this context to gather dependencies if
/// they need to instantiate struct instances or actors.
pub struct E3RequestContext {
    pub e3_id: E3id,
    pub keyshare: Option<Addr<Keyshare>>,
    pub fhe: Option<Arc<Fhe>>,
    pub plaintext: Option<Addr<PlaintextAggregator>>,
    pub publickey: Option<Addr<PublicKeyAggregator>>,
    pub meta: Option<CommitteeMeta>,
    pub store: Repository<E3RequestContextSnapshot>,
}

#[derive(Serialize, Deserialize)]
pub struct E3RequestContextSnapshot {
    pub keyshare: bool,
    pub e3_id: E3id,
    pub fhe: bool,
    pub plaintext: bool,
    pub publickey: bool,
    pub meta: bool,
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
            meta: self.meta.is_some(),
            fhe: self.fhe.is_some(),
            publickey: self.publickey.is_some(),
            plaintext: self.plaintext.is_some(),
            keyshare: self.keyshare.is_some(),
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
    fn repository(&self) -> &Repository<E3RequestContextSnapshot> {
        &self.store
    }
}
