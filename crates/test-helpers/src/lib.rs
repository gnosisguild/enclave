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
use e3_ciphernode_builder::{CiphernodeHandle, EventSystem};
use e3_events::{
    BusHandle, CiphernodeAdded, EnclaveEvent, EnclaveEventData, EventBus, EventBusConfig,
    EventPublisher, HistoryCollector, Seed, Subscribe,
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

/// Sets up an in-memory event environment and cryptographic test fixtures for use in tests.
///
/// This creates an in-memory `EventBus` and two `HistoryCollector`s (one subscribed to all events,
/// the other subscribed to `EnclaveError`), a deterministic shared RNG and seed, BFV parameters
/// (using the provided `param_set` or a secure default), and a deserialized `CommonRandomPoly`.
///
/// # Parameters
///
/// * `param_set` - Optional BFV parameter set to use; when `None`, a default `InsecureSet2048_1032193_1` is used.
///
/// # Returns
///
/// A tuple containing:
/// 1. `BusHandle` — an in-memory event system handle bound to the created event bus,
/// 2. `SharedRng` — a thread-safe, seeded ChaCha20 RNG,
/// 3. `Seed` — seed material derived deterministically,
/// 4. `Arc<BfvParameters>` — the BFV parameters derived from the chosen parameter set,
/// 5. `CommonRandomPoly` — the deserialized common random polynomial created from CRP bytes,
/// 6. `Addr<HistoryCollector<EnclaveEvent>>` — the address of the error history collector,
/// 7. `Addr<HistoryCollector<EnclaveEvent>>` — the address of the general history collector.
///
/// # Examples
///
/// ```
/// let (handle, _rng, _seed, _params, _crp, errors, history) =
///     get_common_setup(None).expect("setup should succeed");
/// // collectors should be running and addresses valid
/// assert!(errors.connected());
/// assert!(history.connected());
/// ```
pub fn get_common_setup(
    param_set: Option<BfvParamSet>,
) -> Result<(
    BusHandle,
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
    let handle = EventSystem::in_mem("cn1").with_event_bus(bus).handle()?;
    Ok((handle, rng, seed, params, crpoly, errors, history))
}

/// Wire node event buses so broadcastable events are forwarded between distinct nodes to simulate a libp2p-like network.
///
/// For each pair of distinct ciphernode handles, this forwards events from the source bus to the destination bus when the event
/// is considered forwardable by `NetEventTranslator` or is a document-publisher event. Events are not forwarded to the same node's bus.
/// This function does not modify nodes or perform network IO; it only connects in-memory event buses so tests can observe cross-node propagation.
///
/// # Examples
///
/// ```no_run
/// // Given a list of initialized CiphernodeHandle values, connect their buses so broadcastable events propagate between them.
/// // let nodes: Vec<CiphernodeHandle> = ...;
/// // simulate_libp2p_net(&nodes);
/// ```
pub fn simulate_libp2p_net(nodes: &[CiphernodeHandle]) {
    for node in nodes.iter() {
        let source = node.bus();
        for (_, node) in nodes.iter().enumerate() {
            let dest = node.bus();
            if source != dest {
                source.pipe_to(dest, |e: &EnclaveEvent| {
                    // TODO: Document publisher events need to be
                    // converted to DocumentReceived events

                    NetEventTranslator::is_forwardable_event(e)
                        || DocumentPublisher::is_document_publisher_event(e)
                });
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
    bus: BusHandle,
    count: usize,
    chain_id: u64,
}

impl AddToCommittee {
    pub fn new(bus: &BusHandle, chain_id: u64) -> Self {
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

        self.bus.publish(evt.clone())?;

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