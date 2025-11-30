// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pub mod application;
pub mod ciphernode_system;
mod plaintext_writer;
mod public_key_writer;
pub mod usecase_helpers;
mod utils;
use actix::prelude::*;
use alloy::primitives::Address;
use anyhow::*;
use e3_ciphernode_builder::CiphernodeHandle;
use e3_events::{
    CiphernodeAdded, EnclaveEvent, EnclaveEventData, EventBus, EventBusConfig, EventDispatcher,
    BusHandle, HistoryCollector, Seed, Subscribe,
};
use e3_fhe::{create_crp, setup_crp_params, ParamsWithCrp};
use e3_net::{DocumentPublisher, NetEventTranslator};
use e3_sdk::bfv_helpers::{BfvParamSet, BfvParamSets};
use e3_utils::SharedRng;
use fhe::bfv::{BfvParameters, Ciphertext, Encoding, Plaintext, PublicKey};
use fhe::mbfv::CommonRandomPoly;
use fhe_traits::{FheEncoder, FheEncrypter};
pub use plaintext_writer::*;
pub use public_key_writer::*;
use rand::Rng;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha20Rng;
use std::sync::Arc;
pub use utils::*;

pub fn create_shared_rng_from_u64(value: u64) -> Arc<std::sync::Mutex<ChaCha20Rng>> {
    Arc::new(std::sync::Mutex::new(ChaCha20Rng::seed_from_u64(value)))
}

pub fn create_seed_from_u64(value: u64) -> Seed {
    Seed(ChaCha20Rng::seed_from_u64(value).get_seed())
}

pub fn create_rng_from_seed(seed: Seed) -> SharedRng {
    Arc::new(std::sync::Mutex::new(ChaCha20Rng::from_seed(seed.into())))
}

pub fn create_crp_from_seed(params: &Arc<BfvParameters>, seed: &Seed) -> Result<CommonRandomPoly> {
    let rng = create_rng_from_seed(seed.clone());
    Ok(create_crp(params.clone(), rng))
}

pub fn create_crp_bytes_params(
    moduli: &[u64],
    degree: usize,
    plaintext_modulus: u64,
    seed: &Seed,
) -> (Vec<u8>, Arc<BfvParameters>) {
    let ParamsWithCrp {
        crp_bytes, params, ..
    } = setup_crp_params(
        moduli,
        degree,
        plaintext_modulus,
        Arc::new(std::sync::Mutex::new(ChaCha20Rng::from_seed(
            seed.clone().into(),
        ))),
    );
    (crp_bytes, params)
}

pub fn get_common_setup(
    param_set: Option<BfvParamSet>,
) -> Result<(
    BusHandle<EnclaveEvent>,
    SharedRng,
    Seed,
    Arc<BfvParameters>,
    CommonRandomPoly,
    Addr<HistoryCollector<EnclaveEvent>>,
    Addr<HistoryCollector<EnclaveEvent>>,
)> {
    let bus = EventBus::<EnclaveEvent>::new(EventBusConfig { deduplicate: true }).start();
    let errors = HistoryCollector::<EnclaveEvent>::new().start();
    let history = HistoryCollector::<EnclaveEvent>::new().start();
    bus.do_send(Subscribe::new("*", history.clone().recipient()));
    bus.do_send(Subscribe::new("EnclaveError", errors.clone().recipient()));

    let rng = create_shared_rng_from_u64(42);
    let seed = create_seed_from_u64(123);
    let param_set = param_set.unwrap_or(BfvParamSets::InsecureSet2048_1032193_1.into());
    let degree = param_set.degree;
    let plaintext_modulus = param_set.plaintext_modulus;
    let moduli = param_set.moduli;
    let (crp_bytes, params) = create_crp_bytes_params(moduli, degree, plaintext_modulus, &seed);
    let crpoly = CommonRandomPoly::deserialize(&crp_bytes.clone(), &params)?;

    Ok((bus.into(), rng, seed, params, crpoly, errors, history))
}

/// Simulate libp2p by taking output events on each local bus and filter for !is_local_only() and forward remaining events back to the event bus
/// deduplication will remove previously seen events.
/// This sets up a set of cyphernodes without libp2p.
/// The way it works is that it feeds back all events from
/// all nodes filteres by whether they are broadcastible or not
/// ```txt
///
///                    ┌─────┐
///                    │ BUS │
///                    └─────┘
///                       │
///          ┌────────────┼────────────┐
///          │            │            │
///          ▼            ▼            ▼
///       ┌────┐       ┌────┐       ┌────┐               
///       │ B1 │       │ B2 │       │ B3 │◀──┐
///       └────┘       └────┘       └────┘   │
///          │            │            │     │
///          │            │            │     │
///          └────────────┼────────────┘     │
///                       │                  │
///                       ▼                  │
///                    ┌─────┐               │               
///                    │ FIL │───────────────┘                 
///                    └─────┘
/// ```
pub fn simulate_libp2p_net(nodes: &[CiphernodeHandle]) {
    for node in nodes.iter() {
        let source = &node.bus();
        for (_, node) in nodes.iter().enumerate() {
            let dest = &node.bus();
            if source != dest {
                EventBus::pipe_filter(
                    source,
                    move |e: &EnclaveEvent| {
                        // TODO: Document publisher events need to be
                        // converted to DocumentReceived events

                        NetEventTranslator::is_forwardable_event(e)
                            || DocumentPublisher::is_document_publisher_event(e)
                    },
                    dest,
                )
            } else {
                println!("not piping bus to itself");
            }
        }
    }
}

/// Creates test eth addresses
/// NOTE: THESE ARE NOT ACTUAL ADDRESSES JUST RANDOM DATA
pub fn create_random_eth_addrs(how_many: u32) -> Vec<String> {
    (0..how_many)
        .map(|_| Address::from_slice(&rand::thread_rng().gen::<[u8; 20]>()).to_string())
        .collect()
}

/// Test helper to add addresses to the committee by creating events on the event bus
#[derive(Clone, Debug)]
pub struct AddToCommittee {
    bus: BusHandle<EnclaveEvent>,
    count: usize,
    chain_id: u64,
}

impl AddToCommittee {
    pub fn new(bus: &BusHandle<EnclaveEvent>, chain_id: u64) -> Self {
        Self {
            bus: bus.clone(),
            chain_id,
            count: 0,
        }
    }
    pub async fn add(&mut self, address: &str) -> Result<EnclaveEventData> {
        let evt = CiphernodeAdded {
            chain_id: self.chain_id,
            address: address.to_owned(),
            index: self.count,
            num_nodes: self.count + 1,
        };

        self.count += 1;

        self.bus.dispatch(evt.clone());

        Ok(evt.into())
    }
}

pub fn encrypt_ciphertext(
    params: &Arc<BfvParameters>,
    pubkey: PublicKey,
    raw_plaintext: Vec<Vec<u64>>,
) -> Result<(Vec<Ciphertext>, Vec<Plaintext>)> {
    let mut rng = ChaCha20Rng::seed_from_u64(42);
    let plaintext: Vec<_> = raw_plaintext
        .into_iter()
        .map(|raw| Ok(Plaintext::try_encode(&raw, Encoding::poly(), &params)?))
        .collect::<Result<_>>()?;

    let ciphertext = plaintext
        .iter()
        .map(|pt| {
            pubkey
                .try_encrypt(&pt, &mut rng)
                .map_err(|e| anyhow!("{e}"))
        })
        .collect::<Result<Vec<Ciphertext>>>()?;
    Ok((ciphertext, plaintext))
}

fn pad_end(input: &[u64], pad: u64, total: usize) -> Vec<u64> {
    let len = input.len();
    let mut cop = input.to_vec();
    cop.extend(std::iter::repeat(pad).take(total - len));
    cop
}
