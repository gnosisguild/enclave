use crate::{E3Feature, E3RequestContext, E3RequestContextSnapshot};
use actix::{Actor, Addr};
use aggregator::{
    PlaintextAggregator, PlaintextAggregatorParams, PlaintextAggregatorState, PublicKeyAggregator,
    PublicKeyAggregatorParams, PublicKeyAggregatorState,
};
use async_trait::async_trait;
use data::{Snapshot, WithPrefix};
use enclave_core::{E3Requested, EnclaveEvent, EventBus};
use fhe::{Fhe, SharedRng};
use keyshare::{Keyshare, KeyshareParams};
use sortition::Sortition;
use std::sync::Arc;

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
    fn event(&self, ctx: &mut crate::E3RequestContext, evt: &EnclaveEvent) {
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
        let fhe_id = format!("//fhe/{e3_id}");
        let fhe = Arc::new(Fhe::from_encoded(&params, seed, self.rng.clone()).unwrap());
        ctx.get_store().at(&fhe_id).write(fhe.snapshot());

        let _ = ctx.set_fhe(fhe);
    }
    async fn hydrate(&self, _ctx: &mut E3RequestContext, _snapshot: &E3RequestContextSnapshot) {}
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
    fn event(&self, ctx: &mut E3RequestContext, evt: &EnclaveEvent) {
        // Save Ciphernode on CiphernodeSelected
        let EnclaveEvent::CiphernodeSelected { data, .. } = evt else {
            return;
        };

        let Some(fhe) = ctx.get_fhe() else {
            return;
        };

        let e3_id = data.clone().e3_id;

        let ks_id = format!("//keystore/{e3_id}");

        let _ = ctx.set_keyshare(
            Keyshare::new(KeyshareParams {
                bus: self.bus.clone(),
                store: ctx.get_store().at(&ks_id),
                fhe: fhe.clone(),
                address: self.address.clone(),
            })
            .start(),
        );
    }

    async fn hydrate(&self, _ctx: &mut E3RequestContext, _snapshot: &E3RequestContextSnapshot) {}
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
    fn event(&self, ctx: &mut E3RequestContext, evt: &EnclaveEvent) {
        // Save plaintext aggregator
        let EnclaveEvent::CiphertextOutputPublished { data, .. } = evt else {
            return;
        };
        let Some(fhe) = ctx.get_fhe() else {
            return;
        };
        let Some(ref meta) = ctx.get_meta() else {
            return;
        };

        let e3_id = data.e3_id.clone();

        let id = &format!("//plaintext/{e3_id}");

        let _ = ctx.set_plaintext(
            PlaintextAggregator::new(
                PlaintextAggregatorParams {
                    fhe: fhe.clone(),
                    bus: self.bus.clone(),
                    store: ctx.get_store().at(id),
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
    async fn hydrate(&self, _ctx: &mut E3RequestContext, _snapshot: &E3RequestContextSnapshot) {}
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
    fn event(&self, ctx: &mut E3RequestContext, evt: &EnclaveEvent) {
        // Saving the publickey aggregator with deps on E3Requested
        let EnclaveEvent::E3Requested { data, .. } = evt else {
            return;
        };

        let Some(fhe) = ctx.get_fhe() else {
            println!("fhe was not on ctx");
            return;
        };
        let Some(ref meta) = ctx.get_meta() else {
            println!("meta was not on ctx");
            return;
        };

        let e3_id = data.e3_id.clone();
        let id = &format!("//publickey/{e3_id}");

        let _ = ctx.set_publickey(
            PublicKeyAggregator::new(
                PublicKeyAggregatorParams {
                    fhe: fhe.clone(),
                    bus: self.bus.clone(),
                    store: ctx.get_store().at(id),
                    sortition: self.sortition.clone(),
                    e3_id,
                    src_chain_id: meta.src_chain_id,
                },
                PublicKeyAggregatorState::init(meta.threshold_m, meta.seed),
            )
            .start(),
        );
    }

    async fn hydrate(&self, _ctx: &mut E3RequestContext, _snapshot: &E3RequestContextSnapshot) {}
}
