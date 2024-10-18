use crate::{E3Feature, E3RequestContext, E3RequestContextSnapshot};
use actix::{Actor, Addr};
use aggregator::{
    PlaintextAggregator, PlaintextAggregatorParams, PlaintextAggregatorRepositoryFactory,
    PlaintextAggregatorState, PublicKeyAggregator, PublicKeyAggregatorParams,
    PublicKeyAggregatorRepositoryFactory, PublicKeyAggregatorState,
};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use data::{Checkpoint, FromSnapshotWithParams, Repository, Snapshot};
use enclave_core::{BusError, E3Requested, EnclaveErrorType, EnclaveEvent, EventBus};
use fhe::{Fhe, FheRepositoryFactory, SharedRng};
use keyshare::{Keyshare, KeyshareParams, KeyshareRepositoryFactory};
use sortition::Sortition;
use std::sync::Arc;

/// TODO: move these to each package with access on MyStruct::launcher()
pub struct FheFeature {
    rng: SharedRng,
}

impl FheFeature {
    pub fn create(rng: SharedRng) -> Box<Self> {
        Box::new(Self { rng })
    }
}

#[async_trait]
impl E3Feature for FheFeature {
    fn on_event(&self, ctx: &mut crate::E3RequestContext, evt: &EnclaveEvent) {
        // Saving the fhe on Committee Requested
        let EnclaveEvent::E3Requested { data, .. } = evt else {
            return;
        };

        let E3Requested {
            params,
            seed,
            e3_id,
            ..
        } = data.clone();

        // FHE doesn't implement Checkpoint so we are going to store it manually
        let fhe = Arc::new(Fhe::from_encoded(&params, seed, self.rng.clone()).unwrap());
        ctx.get_store().store().fhe(&e3_id).write(&fhe.snapshot());
        let _ = ctx.set_fhe(fhe);
    }

    async fn hydrate(
        &self,
        ctx: &mut E3RequestContext,
        snapshot: &E3RequestContextSnapshot,
    ) -> Result<()> {
        // No ID on the snapshot -> bail
        if !snapshot.fhe {
            return Ok(());
        };

        // No Snapshot returned from the store -> bail
        let Some(snap) = ctx.get_store().store().fhe(&ctx.e3_id).read().await? else {
            return Ok(());
        };

        let value = Arc::new(Fhe::from_snapshot(self.rng.clone(), snap).await?);
        ctx.set_fhe(value);

        Ok(())
    }
}

pub struct KeyshareFeature {
    bus: Addr<EventBus>,
    address: String,
}

impl KeyshareFeature {
    pub fn create(bus: Addr<EventBus>, address: &str) -> Box<Self> {
        Box::new(Self {
            bus,
            address: address.to_owned(),
        })
    }
}

#[async_trait]
impl E3Feature for KeyshareFeature {
    fn on_event(&self, ctx: &mut E3RequestContext, evt: &EnclaveEvent) {
        // Save Ciphernode on CiphernodeSelected
        let EnclaveEvent::CiphernodeSelected { data, .. } = evt else {
            return;
        };

        let Some(fhe) = ctx.get_fhe() else {
            self.bus.err(EnclaveErrorType::KeyGeneration, anyhow!("Could not create Keyshare because the fhe instance it depends on was not set on the context."));
            return;
        };

        let e3_id = data.clone().e3_id;

        let _ = ctx.set_keyshare(
            Keyshare::new(KeyshareParams {
                bus: self.bus.clone(),
                store: ctx.get_store().store().keyshare(&e3_id),
                fhe: fhe.clone(),
                address: self.address.clone(),
            })
            .start(),
        );
    }

    async fn hydrate(
        &self,
        ctx: &mut E3RequestContext,
        snapshot: &E3RequestContextSnapshot,
    ) -> Result<()> {
        // No ID on the snapshot -> bail
        if !snapshot.keyshare {
            return Ok(());
        };

        let store = ctx.get_store().store().keyshare(&snapshot.e3_id);

        // No Snapshot returned from the store -> bail
        let Some(snap) = store.read().await? else {
            return Ok(());
        };

        // Get deps
        let Some(fhe) = ctx.fhe.clone() else {
            return Ok(());
        };

        // Construct from snapshot
        let value = Keyshare::from_snapshot(
            KeyshareParams {
                fhe,
                bus: self.bus.clone(),
                store,
                address: self.address.clone(),
            },
            snap,
        )
        .await?
        .start();

        // send to context
        ctx.set_keyshare(value);

        Ok(())
    }
}

pub struct PlaintextAggregatorFeature {
    bus: Addr<EventBus>,
    sortition: Addr<Sortition>,
}
impl PlaintextAggregatorFeature {
    pub fn create(bus: Addr<EventBus>, sortition: Addr<Sortition>) -> Box<Self> {
        Box::new(Self { bus, sortition })
    }
}

#[async_trait]
impl E3Feature for PlaintextAggregatorFeature {
    fn on_event(&self, ctx: &mut E3RequestContext, evt: &EnclaveEvent) {
        // Save plaintext aggregator
        let EnclaveEvent::CiphertextOutputPublished { data, .. } = evt else {
            return;
        };
        let Some(fhe) = ctx.get_fhe() else {
            self.   bus.err(EnclaveErrorType::PlaintextAggregation, anyhow!("Could not create PlaintextAggregator because the fhe instance it depends on was not set on the context."));

            return;
        };
        let Some(ref meta) = ctx.get_meta() else {
            self. bus.err(EnclaveErrorType::PlaintextAggregation, anyhow!("Could not create PlaintextAggregator because the meta instance it depends on was not set on the context."));

            return;
        };

        let e3_id = data.e3_id.clone();

        let _ = ctx.set_plaintext(
            PlaintextAggregator::new(
                PlaintextAggregatorParams {
                    fhe: fhe.clone(),
                    bus: self.bus.clone(),
                    store: ctx.get_store().store().plaintext(&e3_id),
                    sortition: self.sortition.clone(),
                    e3_id,
                    src_chain_id: meta.src_chain_id,
                },
                PlaintextAggregatorState::init(
                    meta.threshold_m,
                    meta.seed,
                    data.ciphertext_output.clone(),
                ),
            )
            .start(),
        );
    }

    async fn hydrate(
        &self,
        ctx: &mut E3RequestContext,
        snapshot: &E3RequestContextSnapshot,
    ) -> Result<()> {
        // No ID on the snapshot -> bail
        if !snapshot.plaintext {
            return Ok(());
        }

        let store = ctx.get_store().store().plaintext(&snapshot.e3_id);

        // No Snapshot returned from the store -> bail
        let Some(snap) = store.read().await? else {
            return Ok(());
        };

        // Get deps
        let Some(fhe) = ctx.fhe.clone() else {
            return Ok(());
        };

        let Some(meta) = ctx.meta.clone() else {
            return Ok(());
        };

        let value = PlaintextAggregator::from_snapshot(
            PlaintextAggregatorParams {
                fhe: fhe.clone(),
                bus: self.bus.clone(),
                store,
                sortition: self.sortition.clone(),
                e3_id: ctx.e3_id.clone(),
                src_chain_id: meta.src_chain_id,
            },
            snap,
        )
        .await?
        .start();

        // send to context
        ctx.set_plaintext(value);

        Ok(())
    }
}

pub struct PublicKeyAggregatorFeature {
    bus: Addr<EventBus>,
    sortition: Addr<Sortition>,
}

impl PublicKeyAggregatorFeature {
    pub fn create(bus: Addr<EventBus>, sortition: Addr<Sortition>) -> Box<Self> {
        Box::new(Self { bus, sortition })
    }
}

#[async_trait]
impl E3Feature for PublicKeyAggregatorFeature {
    fn on_event(&self, ctx: &mut E3RequestContext, evt: &EnclaveEvent) {
        // Saving the publickey aggregator with deps on E3Requested
        let EnclaveEvent::E3Requested { data, .. } = evt else {
            return;
        };

        let Some(fhe) = ctx.get_fhe() else {
            self.bus.err(EnclaveErrorType::PublickeyAggregation, anyhow!("Could not create PublicKeyAggregator because the fhe instance it depends on was not set on the context."));
            return;
        };
        let Some(ref meta) = ctx.get_meta() else {
            self.bus.err(EnclaveErrorType::PublickeyAggregation, anyhow!("Could not create PublicKeyAggregator because the meta instance it depends on was not set on the context."));
            return;
        };

        let e3_id = data.e3_id.clone();

        let _ = ctx.set_publickey(
            PublicKeyAggregator::new(
                PublicKeyAggregatorParams {
                    fhe: fhe.clone(),
                    bus: self.bus.clone(),
                    store: ctx.get_store().store().publickey(&e3_id),
                    sortition: self.sortition.clone(),
                    e3_id,
                    src_chain_id: meta.src_chain_id,
                },
                PublicKeyAggregatorState::init(meta.threshold_m, meta.seed),
            )
            .start(),
        );
    }

    async fn hydrate(
        &self,
        ctx: &mut E3RequestContext,
        snapshot: &E3RequestContextSnapshot,
    ) -> Result<()> {
        // No ID on the snapshot -> bail
        if !snapshot.publickey {
            return Ok(());
        };

        let repository = ctx.get_store().store().publickey(&ctx.e3_id);

        // No Snapshot returned from the store -> bail
        let Some(snap) = repository.read().await? else {
            return Ok(());
        };

        // Get deps
        let Some(fhe) = ctx.fhe.clone() else {
            return Ok(());
        };

        let Some(meta) = ctx.meta.clone() else {
            return Ok(());
        };

        let value = PublicKeyAggregator::from_snapshot(
            PublicKeyAggregatorParams {
                fhe: fhe.clone(),
                bus: self.bus.clone(),
                store: repository,
                sortition: self.sortition.clone(),
                e3_id: ctx.e3_id.clone(),
                src_chain_id: meta.src_chain_id,
            },
            snap,
        )
        .await?
        .start();

        // send to context
        ctx.set_publickey(value);

        Ok(())
    }
}
