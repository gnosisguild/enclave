use crate::EventHook;
use actix::{Actor, Addr};
use aggregator::{PlaintextAggregator, PublicKeyAggregator};
use data::{DataStore, WithPrefix};
use enclave_core::{E3Requested, EnclaveEvent, EventBus};
use fhe::{Fhe, SharedRng};
use keyshare::Keyshare;
use sortition::Sortition;
use std::sync::Arc;

pub struct LazyFhe;

impl LazyFhe {
    pub fn create(rng: SharedRng) -> EventHook {
        Box::new(move |ctx, evt| {
            // Saving the fhe on Committee Requested
            let EnclaveEvent::E3Requested { data, .. } = evt else {
                return;
            };

            let E3Requested {
                params,
                seed,
                e3_id,
                ..
            } = data;

            // Set the FHE instance passing in the instance id
            let _ = ctx.set_fhe(
                &format!("//fhe/{e3_id}"),
                Arc::new(Fhe::from_encoded(&params, seed, rng.clone()).unwrap()),
            );
        })
    }
}

pub struct LazyKeyshare;
impl LazyKeyshare {
    pub fn create(bus: Addr<EventBus>, address: &str) -> EventHook {
        let address = address.to_string();
        Box::new(move |ctx, evt| {
            // Save Ciphernode on CiphernodeSelected
            let EnclaveEvent::CiphernodeSelected { data, .. } = evt else {
                return;
            };

            let Some(fhe) = ctx.get_fhe() else {
                return;
            };

            let e3_id = data.e3_id;

            let ks_id = &format!("//keystore/{e3_id}");

            let _ = ctx.set_keyshare(
                ks_id,
                Keyshare::new(
                    bus.clone(),
                    ctx.store.clone().base(ks_id),
                    fhe.clone(),
                    &address,
                )
                .start(),
            );
        })
    }
}

pub struct LazyPlaintextAggregator;
impl LazyPlaintextAggregator {
    pub fn create(bus: Addr<EventBus>, sortition: Addr<Sortition>) -> EventHook {
        Box::new(move |ctx, evt| {
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

            let e3_id = data.e3_id;

            let id = &format!("//plaintext/{e3_id}");

            let _ = ctx.set_plaintext(
                id,
                PlaintextAggregator::new(
                    fhe.clone(),
                    bus.clone(),
                    sortition.clone(),
                    e3_id,
                    meta.threshold_m,
                    meta.seed,
                    data.ciphertext_output,
                    meta.src_chain_id,
                )
                .start(),
            );
        })
    }
}

pub struct LazyPublicKeyAggregator;
impl LazyPublicKeyAggregator {
    pub fn create(bus: Addr<EventBus>, sortition: Addr<Sortition>) -> EventHook {
        Box::new(move |ctx, evt| {
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

            let e3_id = data.e3_id;
            let id = &format!("//publickey/{e3_id}");

            let _ = ctx.set_publickey(
                id,
                PublicKeyAggregator::new(
                    fhe.clone(),
                    bus.clone(),
                    sortition.clone(),
                    e3_id,
                    meta.threshold_m,
                    meta.seed,
                    meta.src_chain_id,
                )
                .start(),
            );
        })
    }
}
