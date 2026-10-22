// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Shared smudging-noise helpers for the BFV parameter presets.

use crate::BfvPreset;
use fhe::bfv::BfvParameters;
use fhe::trbfv::{
    Lambda, SmudgingBoundCalculator, SmudgingBoundCalculatorConfig, SmudgingNoise,
    SmudgingNoiseGenerator,
};
use fhe_math::rq::sample_uniform_coefficients_bigint;
use num_bigint::BigInt;
use num_bigint::BigUint;
use rand::{CryptoRng, RngCore};
use std::sync::Arc;

/// Arguments that produce the fresh-noise bound checked by C6.
///
/// `num_ciphertexts` is the number of fresh ciphertexts summed into one ciphertext.
/// `num_parties` is the committee size. Both values match the bound compiled for that preset.
pub struct FreshSmudgingInputs {
    pub num_parties: usize,
    pub num_ciphertexts: usize,
    pub mult_depth: u32,
    pub lambda: usize,
}

/// Return the smudging inputs for one threshold preset and committee size.
pub fn fresh_smudging_inputs(
    preset: BfvPreset,
    num_parties: usize,
) -> Result<FreshSmudgingInputs, String> {
    let defaults = preset
        .search_defaults()
        .ok_or_else(|| format!("{preset:?} has no search defaults"))?;
    let lambda = preset.metadata().lambda;
    Ok(FreshSmudgingInputs {
        num_parties,
        num_ciphertexts: usize::try_from(defaults.z).map_err(|_| {
            format!(
                "{preset:?} ciphertext count {} does not fit usize",
                defaults.z
            )
        })?,
        // The paired presets are summation-only; multiplication is not in their noise model.
        mult_depth: 0,
        lambda,
    })
}

/// Sample one fresh smudging polynomial with the operating-system generator.
pub fn sample_fresh_smudging_error(
    params: Arc<BfvParameters>,
    n: usize,
    num_ciphertexts: usize,
    mult_depth: u32,
    lambda: usize,
) -> Result<Vec<BigInt>, fhe::Error> {
    let mut rng = rand::rng();
    generate_smudging_error(params, n, num_ciphertexts, mult_depth, lambda, &mut rng)
}

/// Generate centered smudging coefficients for one threshold-BFV operation.
///
/// The circuit witness keeps these integers. [`smudging_noise_from_signed_coefficients`] wraps
/// the same integers for one synchronized partial decryption.
pub fn generate_smudging_error<R: RngCore + CryptoRng>(
    params: Arc<BfvParameters>,
    n: usize,
    num_ciphertexts: usize,
    mult_depth: u32,
    lambda: usize,
    rng: &mut R,
) -> Result<Vec<BigInt>, fhe::Error> {
    let bound = BigInt::from(calculate_smudging_bound(
        params.clone(),
        n,
        num_ciphertexts,
        mult_depth,
        lambda,
    )?);
    Ok(sample_uniform_coefficients_bigint(
        &bound,
        params.degree(),
        rng,
    ))
}

/// Wrap one sampled coefficient vector as synchronized decryption noise.
pub fn smudging_noise_from_signed_coefficients(
    params: Arc<BfvParameters>,
    n: usize,
    num_ciphertexts: usize,
    mult_depth: u32,
    lambda: usize,
    coeffs: &[BigInt],
) -> Result<SmudgingNoise, fhe::Error> {
    let generator = SmudgingNoiseGenerator::new(bound_config(
        params,
        n,
        num_ciphertexts,
        mult_depth,
        lambda,
    )?)?;
    generator.wrap_signed_coefficients(coeffs)
}

/// Calculate the smudging bound for one threshold-BFV operation.
pub fn calculate_smudging_bound(
    params: Arc<BfvParameters>,
    n: usize,
    num_ciphertexts: usize,
    mult_depth: u32,
    lambda: usize,
) -> Result<BigUint, fhe::Error> {
    Ok(SmudgingBoundCalculator::new(bound_config(
        params,
        n,
        num_ciphertexts,
        mult_depth,
        lambda,
    )?)
    .calculate_sm_bound()?)
}

fn bound_config(
    params: Arc<BfvParameters>,
    n: usize,
    num_ciphertexts: usize,
    mult_depth: u32,
    lambda: usize,
) -> Result<SmudgingBoundCalculatorConfig, fhe::Error> {
    let lambda = Lambda::secure(lambda).unwrap_or_else(|_| Lambda::insecure(lambda));
    SmudgingBoundCalculatorConfig::new_multiplicative(
        params,
        n,
        num_ciphertexts,
        mult_depth,
        lambda,
    )
}
