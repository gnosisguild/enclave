// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

mod plaintext_writer;
mod public_key_writer;
mod utils;

use std::sync::Arc;

use actix::prelude::*;
use anyhow::*;
use e3_events::{
    EnclaveEvent, ErrorCollector, EventBus, EventBusConfig, HistoryCollector, Seed, Subscribe,
};
use e3_fhe::{setup_crp_params, ParamsWithCrp, SharedRng};
use e3_sdk::bfv_helpers::params::SET_2048_1032193_1;
use fhe_rs::bfv::BfvParameters;
use fhe_rs::mbfv::CommonRandomPoly;
pub use plaintext_writer::*;
pub use public_key_writer::*;
use rand_chacha::rand_core::SeedableRng;
use rand_chacha::ChaCha20Rng;
pub use utils::*;

pub fn create_shared_rng_from_u64(value: u64) -> Arc<std::sync::Mutex<ChaCha20Rng>> {
    Arc::new(std::sync::Mutex::new(ChaCha20Rng::seed_from_u64(value)))
}

pub fn create_seed_from_u64(value: u64) -> Seed {
    Seed(ChaCha20Rng::seed_from_u64(value).get_seed())
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

pub fn get_common_setup() -> Result<(
    Addr<EventBus<EnclaveEvent>>,
    SharedRng,
    Seed,
    Arc<BfvParameters>,
    CommonRandomPoly,
    Addr<ErrorCollector<EnclaveEvent>>,
    Addr<HistoryCollector<EnclaveEvent>>,
)> {
    let bus = EventBus::<EnclaveEvent>::new(EventBusConfig { deduplicate: true }).start();
    let errors = ErrorCollector::<EnclaveEvent>::new().start();
    let history = HistoryCollector::<EnclaveEvent>::new().start();
    bus.do_send(Subscribe::new("*", history.clone().recipient()));
    bus.do_send(Subscribe::new("EnclaveError", errors.clone().recipient()));

    let rng = create_shared_rng_from_u64(42);
    let seed = create_seed_from_u64(123);
    let (degree, plaintext_modulus, moduli) = SET_2048_1032193_1;
    let (crp_bytes, params) = create_crp_bytes_params(&moduli, degree, plaintext_modulus, &seed);
    let crpoly = CommonRandomPoly::deserialize(&crp_bytes.clone(), &params)?;

    Ok((bus, rng, seed, params, crpoly, errors, history))
}
