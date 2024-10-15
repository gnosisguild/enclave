use crate::EventHook;
use actix::{Actor, Addr};
use aggregator::{PlaintextAggregator, PublicKeyAggregator};
use anyhow::anyhow;
use data::Data;
use enclave_core::{BusError, E3Requested, EnclaveErrorType, EnclaveEvent, EventBus};
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
            let E3Requested { params, seed, .. } = data;

            ctx.fhe = Some(Arc::new(
                Fhe::from_encoded(&params, seed, rng.clone()).unwrap(),
            ));
        })
    }
}

pub struct LazyKeyshare;
impl LazyKeyshare {
    pub fn create(bus: Addr<EventBus>, data: Addr<Data>, address: &str) -> EventHook {
        let address = address.to_string();
        Box::new(move |ctx, evt| {
            // Save Ciphernode on CiphernodeSelected
            let EnclaveEvent::CiphernodeSelected { .. } = evt else {
                return;
            };

            let Some(ref fhe) = ctx.fhe else {
                bus.err(EnclaveErrorType::KeyGeneration, anyhow!("Could not create Keyshare because the fhe instance it depends on was not set on the context."));
                return;
            };

            ctx.keyshare =
                Some(Keyshare::new(bus.clone(), data.clone(), fhe.clone(), &address).start())
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
            let Some(ref fhe) = ctx.fhe else {
                bus.err(EnclaveErrorType::PlaintextAggregation, anyhow!("Could not create PlaintextAggregator because the fhe instance it depends on was not set on the context."));
                return;
            };
            let Some(ref meta) = ctx.meta else {
                bus.err(EnclaveErrorType::PlaintextAggregation, anyhow!("Could not create PlaintextAggregator because the meta instance it depends on was not set on the context."));
                return;
            };

            ctx.plaintext = Some(
                PlaintextAggregator::new(
                    fhe.clone(),
                    bus.clone(),
                    sortition.clone(),
                    data.e3_id,
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

            let Some(ref fhe) = ctx.fhe else {
                bus.err(EnclaveErrorType::PublickeyAggregation, anyhow!("Could not create PublicKeyAggregator because the fhe instance it depends on was not set on the context."));
                return;
            };
            let Some(ref meta) = ctx.meta else {
                bus.err(EnclaveErrorType::PublickeyAggregation, anyhow!("Could not create PublicKeyAggregator because the meta instance it depends on was not set on the context."));
                return;
            };

            ctx.publickey = Some(
                PublicKeyAggregator::new(
                    fhe.clone(),
                    bus.clone(),
                    sortition.clone(),
                    data.e3_id,
                    meta.threshold_m,
                    meta.seed,
                    meta.src_chain_id,
                )
                .start(),
            );
        })
    }
}
