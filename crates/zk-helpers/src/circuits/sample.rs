// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Sample data generation for circuits.
//!
//! [`Sample`] produces a random BFV key pair; the public key is used as input
//! for codegen and tests (e.g. pk circuit). For share-computation, it can
//! also produce secret-key or smudging-noise data: secret in CRT form, Shamir
//! shares, and parity matrices. Smudging noise is generated with the computed
//! smudging bound (error_size) so coefficients are non-zero.

use crate::ciphernodes_committee::CiphernodesCommittee;
use crate::ciphernodes_committee::CiphernodesCommitteeSize;
use crate::computation::DkgInputType;
use crate::CircuitsErrors;
use e3_fhe_params::build_pair_for_preset;
use e3_fhe_params::BfvPreset;
use e3_parity_matrix::build_generator_matrix;
use e3_parity_matrix::{null_space, ParityMatrix, ParityMatrixConfig};
use e3_polynomial::CrtPolynomial;
use fhe::bfv::{BfvParameters, PublicKey, SecretKey};
use fhe::trbfv::{ShareManager, TRBFV};
use num_bigint::BigInt;
use num_bigint::BigUint;
use rand::thread_rng;
use std::sync::Arc;

/// Shamir secret shares: one limb per CRT modulus (rows = parties, cols = polynomial coefficients).
pub type SecretShares = Vec<ndarray::Array2<BigInt>>;

/// A sample BFV public key (and optionally related data) for circuit codegen or tests.
#[derive(Debug, Clone)]
pub struct Sample {
    /// Committee information.
    pub committee: CiphernodesCommittee,
    /// DKG BFV public key.
    pub dkg_public_key: PublicKey,
    /// Secret in CRT form (SK or smudging noise, depending on [`DkgInputType`]).
    pub secret: Option<CrtPolynomial>,
    /// Secret shares (one [`ndarray::Array2<BigInt>`] per modulus; empty when [`DkgInputType`] is `None`).
    pub secret_sss: SecretShares,
    /// Parity check matrix per modulus (null space of generator), one [`ParityMatrix`] per CRT modulus.
    pub parity_matrix: Vec<ParityMatrix>,
}

impl Sample {
    /// Generates a random secret key and public key for the given BFV parameters.
    pub fn generate(
        threshold_params: &Arc<BfvParameters>,
        dkg_params: &Arc<BfvParameters>,
        dkg_input_type: Option<DkgInputType>,
        committee_size: CiphernodesCommitteeSize,
        num_ciphertexts: u128, // z in the search defaults
        lambda: u32,
    ) -> Result<Self, CircuitsErrors> {
        let mut rng = thread_rng();

        let committee = committee_size.values();

        let dkg_secret_key = SecretKey::random(&dkg_params, &mut rng);
        let dkg_public_key = PublicKey::new(&dkg_secret_key, &mut rng);

        let trbfv = TRBFV::new(committee.n, committee.threshold, threshold_params.clone())
            .map_err(|e| CircuitsErrors::Sample(format!("Failed to create TRBFV: {:?}", e)))?;
        let mut share_manager =
            ShareManager::new(committee.n, committee.threshold, threshold_params.clone());

        // Parity check matrix (null space of generator) per modulus: [L][N_PARTIES-T][N_PARTIES+1].
        let mut parity_matrix = Vec::with_capacity(threshold_params.moduli().len());
        for &qi in threshold_params.moduli() {
            let q = BigUint::from(qi);
            let g = build_generator_matrix(&ParityMatrixConfig {
                q: q.clone(),
                t: committee.threshold,
                n: committee.n,
            })
            .map_err(|e| {
                CircuitsErrors::Sample(format!("Failed to build generator matrix: {:?}", e))
            })?;
            let h = null_space(&g, &q).map_err(|e| {
                CircuitsErrors::Sample(format!("Failed to compute null space: {:?}", e))
            })?;
            parity_matrix.push(h);
        }

        let (secret, secret_sss) = match dkg_input_type {
            Some(DkgInputType::SecretKey) => {
                let threshold_secret_key = SecretKey::random(&threshold_params, &mut rng);

                let sk_poly = share_manager
                    .coeffs_to_poly_level0(threshold_secret_key.coeffs.clone().as_ref())
                    .map_err(|e| {
                        CircuitsErrors::Sample(format!(
                            "Failed to convert SK coeffs to poly: {:?}",
                            e
                        ))
                    })?;

                let sk_sss_u64 = share_manager
                    .generate_secret_shares_from_poly(sk_poly.clone(), rng)
                    .map_err(|e| {
                        CircuitsErrors::Sample(format!("Failed to generate SK shares: {:?}", e))
                    })?;
                let secret_sss: SecretShares = sk_sss_u64
                    .into_iter()
                    .map(|arr| arr.mapv(BigInt::from))
                    .collect();

                let sk_coeffs: Vec<BigInt> = threshold_secret_key
                    .coeffs
                    .iter()
                    .map(|&c| BigInt::from(c))
                    .collect();
                let mut secret_crt =
                    CrtPolynomial::from_bigint_coeffs(&sk_coeffs, threshold_params.moduli());
                secret_crt.center(threshold_params.moduli())?;

                (Some(secret_crt), secret_sss)
            }
            Some(DkgInputType::SmudgingNoise) => {
                let esi_coeffs = trbfv
                    .generate_smudging_error(num_ciphertexts as usize, lambda as usize, &mut rng)
                    .map_err(|e| {
                        CircuitsErrors::Sample(format!(
                            "Failed to generate smudging error: {:?}",
                            e
                        ))
                    })?;
                let esi_poly = share_manager.bigints_to_poly(&esi_coeffs).map_err(|e| {
                    CircuitsErrors::Sample(format!("Failed to convert error to poly: {:?}", e))
                })?;
                let esi_sss_u64 = share_manager
                    .generate_secret_shares_from_poly(esi_poly.clone(), rng)
                    .map_err(|e| {
                        CircuitsErrors::Sample(format!("Failed to generate error shares: {:?}", e))
                    })?;
                let secret_sss: SecretShares = esi_sss_u64
                    .into_iter()
                    .map(|arr| arr.mapv(BigInt::from))
                    .collect();

                let mut secret_crt =
                    CrtPolynomial::from_bigint_coeffs(&esi_coeffs, threshold_params.moduli());
                secret_crt.center(threshold_params.moduli())?;

                (Some(secret_crt), secret_sss)
            }
            None => (None, Vec::new()),
        };

        Ok(Self {
            committee,
            dkg_public_key,
            secret,
            secret_sss,
            parity_matrix,
        })
    }
}

/// Prepares a sample for testing using a threshold preset (DKG params come from its pair).
pub fn prepare_sample_for_test(
    threshold_preset: BfvPreset,
    committee: CiphernodesCommitteeSize,
    dkg_input_type: Option<DkgInputType>,
) -> Result<Sample, CircuitsErrors> {
    let (threshold_params, dkg_params) = build_pair_for_preset(threshold_preset)
        .map_err(|e| CircuitsErrors::Sample(e.to_string()))?;
    let defaults = threshold_preset
        .search_defaults()
        .ok_or_else(|| CircuitsErrors::Sample("preset has no search defaults".to_string()))?;
    let num_ciphertexts = defaults.z;
    let lambda = defaults.lambda;
    let sample = Sample::generate(
        &threshold_params,
        &dkg_params,
        dkg_input_type,
        committee,
        num_ciphertexts,
        lambda,
    )
    .map_err(|e| CircuitsErrors::Sample(e.to_string()))?;
    Ok(sample)
}

#[cfg(test)]
mod tests {
    use crate::ciphernodes_committee::CiphernodesCommitteeSize;
    use crate::computation::DkgInputType;
    use e3_fhe_params::BfvPreset;

    use super::*;

    #[test]
    fn test_generate_secret_key_sample() {
        let committee = CiphernodesCommitteeSize::Small.values();
        let sample = prepare_sample_for_test(
            BfvPreset::InsecureThreshold512,
            CiphernodesCommitteeSize::Small,
            Some(DkgInputType::SecretKey),
        )
        .unwrap();

        assert_eq!(sample.committee.n, committee.n);
        assert_eq!(sample.committee.threshold, committee.threshold);
        assert_eq!(sample.committee.h, committee.h);
        assert_eq!(sample.dkg_public_key.c.c.len(), 2);
        assert_eq!(sample.secret_sss.len(), 2);
        let secret = sample.secret.as_ref().unwrap();
        assert_eq!(secret.limbs.len(), 2);
    }

    #[test]
    fn test_generate_smudging_noise_sample() {
        let committee = CiphernodesCommitteeSize::Small.values();
        let sample = prepare_sample_for_test(
            BfvPreset::InsecureThreshold512,
            CiphernodesCommitteeSize::Small,
            Some(DkgInputType::SmudgingNoise),
        )
        .unwrap();

        assert_eq!(sample.committee.n, committee.n);
        assert_eq!(sample.committee.threshold, committee.threshold);
        assert_eq!(sample.dkg_public_key.c.c.len(), 2);
        assert_eq!(sample.secret_sss.len(), 2);
        let secret = sample.secret.as_ref().unwrap();
        assert_eq!(secret.limbs.len(), 2);
    }
}
