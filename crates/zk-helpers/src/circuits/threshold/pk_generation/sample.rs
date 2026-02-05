// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Sample data generation for pk generation circuit.
//!
//! [`Sample`] produces a random BFV key pair and plaintext; the public key and plaintext are used as input
//! for codegen and tests.

use crate::{
    threshold::pk_generation::PkGenerationCircuitInput, CiphernodesCommittee, CircuitsErrors,
};
use e3_fhe_params::{build_pair_for_preset, BfvPreset};
use e3_polynomial::CrtPolynomial;
use fhe::mbfv::PublicKeyShare;
use fhe::{
    bfv::SecretKey,
    mbfv::CommonRandomPoly,
    trbfv::{ShareManager, TRBFV},
};
use rand::thread_rng;
use std::ops::Deref;

impl PkGenerationCircuitInput {
    pub fn generate_sample(
        preset: BfvPreset,
        committee: CiphernodesCommittee,
    ) -> Result<Self, CircuitsErrors> {
        let (threshold_params, _) = build_pair_for_preset(preset).unwrap();

        let mut rng = thread_rng();

        let secret_key = SecretKey::random(&threshold_params, &mut rng);
        let crp = CommonRandomPoly::new(&threshold_params, &mut rng).unwrap();

        let (public_key_share, a, sk, e) =
            PublicKeyShare::new_extended(&secret_key, crp.clone(), &mut rng).unwrap();

        let num_parties = committee.n;
        let threshold = committee.threshold;
        let preset_metadata = preset.metadata();
        let num_ciphertexts = 1; // We only need one ciphertext for public key generation.

        let trbfv = TRBFV::new(num_parties, threshold, threshold_params.clone())?;
        let share_manager = ShareManager::new(num_parties, threshold, threshold_params);

        // Generate smudging error coefficients
        let esi_coeffs =
            trbfv.generate_smudging_error(num_ciphertexts, preset_metadata.lambda, &mut rng)?;

        // Convert to polynomial in RNS representation
        // bigints_to_poly returns Zeroizing<Poly>, we need to clone the inner Poly
        let e_sm_rns_zeroizing = share_manager.bigints_to_poly(&esi_coeffs)?;

        let e_sm = e_sm_rns_zeroizing.deref().clone();

        Ok(PkGenerationCircuitInput {
            committee,
            pk_share: CrtPolynomial::from_fhe_polynomial(&public_key_share),
            a: CrtPolynomial::from_fhe_polynomial(&a),
            eek: CrtPolynomial::from_fhe_polynomial(&e),
            e_sm: CrtPolynomial::from_fhe_polynomial(&e_sm),
            sk: CrtPolynomial::from_fhe_polynomial(&sk),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        computation::Computation,
        threshold::pk_generation::{PkGenerationCircuitInput, Witness},
        CiphernodesCommitteeSize,
    };

    use e3_fhe_params::DEFAULT_BFV_PRESET;

    #[test]
    fn test_generate_sample() {
        let committee = CiphernodesCommitteeSize::Small.values();
        let sample =
            PkGenerationCircuitInput::generate_sample(DEFAULT_BFV_PRESET, committee).unwrap();
        let witness = Witness::compute(DEFAULT_BFV_PRESET, &sample).unwrap();

        assert_eq!(witness.pk0is.limbs.len(), 2);
        assert_eq!(witness.a.limbs.len(), 2);
        assert_eq!(witness.e_sm.limbs.len(), 2);
        assert_eq!(witness.r1is.limbs.len(), 2);
        assert_eq!(witness.r2is.limbs.len(), 2);
    }
}
