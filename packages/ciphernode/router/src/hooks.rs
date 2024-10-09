use crate::EventHook;
use actix::{Actor, Addr};
use aggregator::{PlaintextAggregator, PublicKeyAggregator};
use data::Data;
use enclave_core::{Die, E3Requested, EnclaveEvent, EventBus};
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
        Box::new(move |ctx, evt| match evt {
            EnclaveEvent::CiphernodeSelected { .. } => {
                let Some(ref fhe) = ctx.fhe else {
                    return;
                };

                ctx.keyshare =
                    Some(Keyshare::new(bus.clone(), data.clone(), fhe.clone(), &address).start())
            }
            EnclaveEvent::E3RequestComplete { .. } => {
                let Some(actor) = ctx.keyshare.take() else {
                    return;
                };

                actor.do_send(Die);
            }
            _ => (),
        })
    }
}

pub struct LazyPlaintextAggregator;
impl LazyPlaintextAggregator {
    pub fn create(bus: Addr<EventBus>, sortition: Addr<Sortition>) -> EventHook {
        Box::new(move |ctx, evt| match evt {
            EnclaveEvent::CiphertextOutputPublished { data, .. } => {
                let Some(ref fhe) = ctx.fhe else {
                    return;
                };
                let Some(ref meta) = ctx.meta else {
                    return;
                };

                // Save plaintext aggregator
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
            }
            EnclaveEvent::E3RequestComplete { .. } => {
                let Some(actor) = ctx.plaintext.take() else {
                    return;
                };

                actor.do_send(Die);
            }
            _ => (),
        })
    }
}

pub struct LazyPublicKeyAggregator;
impl LazyPublicKeyAggregator {
    pub fn create(bus: Addr<EventBus>, sortition: Addr<Sortition>) -> EventHook {
        Box::new(move |ctx, evt| match evt {
            // Saving the publickey aggregator with deps on E3Requested
            EnclaveEvent::E3Requested { data, .. } => {
                let Some(ref fhe) = ctx.fhe else {
                    println!("fhe was not on ctx");
                    return;
                };
                let Some(ref meta) = ctx.meta else {
                    println!("meta was not on ctx");
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
            }
            EnclaveEvent::E3RequestComplete { .. } => {
                let Some(actor) = ctx.publickey.take() else {
                    return;
                };

                actor.do_send(Die);
            }

            _ => (),
        })
    }
}
