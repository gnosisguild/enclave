// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Sample data generation for pk aggregation circuit.
//!
//! [`Sample`] produces a random BFV public key shares from H honest parties and the aggregated public key;
//! the public key shares and aggregated public key are used as input for codegen and tests.

use crate::{
    threshold::pk_aggregation::PkAggregationCircuitInput, CiphernodesCommittee, CircuitsErrors,
};
use e3_fhe_params::{build_pair_for_preset, BfvPreset};
use e3_polynomial::CrtPolynomial;
use fhe::mbfv::{AggregateIter, PublicKeyShare};
use fhe::{
    bfv::{PublicKey, SecretKey},
    mbfv::CommonRandomPoly,
};
use rand::rngs::OsRng;
use rand::thread_rng;

impl PkAggregationCircuitInput {
    pub fn generate_sample(
        preset: BfvPreset,
        committee: CiphernodesCommittee,
    ) -> Result<Self, CircuitsErrors> {
        let (threshold_params, _) = build_pair_for_preset(preset).map_err(|e| {
            CircuitsErrors::Sample(format!("Failed to build pair for preset: {:?}", e))
        })?;

        let mut rng = OsRng;
        let mut thread_rng = thread_rng();

        let crp = CommonRandomPoly::new(&threshold_params, &mut rng)
            .map_err(|e| CircuitsErrors::Sample(format!("Failed to create CRP: {:?}", e)))?;

        // Generate public key shares for each party
        let mut pk_shares = Vec::new();
        let mut pk0_shares = Vec::new();

        for _ in 0..committee.h {
            let sk = SecretKey::random(&threshold_params, &mut rng);
            // Create PublicKeyShare - this generates the p0_share with a specific error term
            let pk_share = PublicKeyShare::new(&sk, crp.clone(), &mut thread_rng).map_err(|e| {
                CircuitsErrors::Sample(format!("Failed to create public key share: {:?}", e))
            })?;

            // Extract the p0_share Poly from the PublicKeyShare
            // This ensures we use the same error term for both aggregation and vector extraction
            let pk0_share = CrtPolynomial::from_fhe_polynomial(&pk_share.p0_share());

            pk_shares.push(pk_share);
            pk0_shares.push(pk0_share);
        }

        // Aggregate public key shares to get the full public key
        let public_key: PublicKey = pk_shares.iter().cloned().aggregate().map_err(|e| {
            CircuitsErrors::Sample(format!("Failed to aggregate public key: {:?}", e))
        })?;

        Ok(PkAggregationCircuitInput {
            committee,
            public_key,
            pk0_shares,
            a: CrtPolynomial::from_fhe_polynomial(&crp.poly()),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        computation::Computation,
        threshold::pk_aggregation::computation::Configs,
        threshold::pk_aggregation::{Inputs, PkAggregationCircuitInput},
        CiphernodesCommitteeSize,
    };

    use e3_fhe_params::BfvPreset;

    #[test]
    fn test_generate_sample() {
        let preset = BfvPreset::InsecureThreshold512;
        let committee = CiphernodesCommitteeSize::Small.values();
        let configs = Configs::compute(preset, &()).unwrap();

        let sample = PkAggregationCircuitInput::generate_sample(preset, committee).unwrap();
        let inputs = Inputs::compute(preset, &sample).unwrap();

        assert_eq!(inputs.pk0.len(), sample.committee.h);
        assert_eq!(inputs.pk1.len(), sample.committee.h);
        assert_eq!(inputs.pk0_agg.limbs.len(), configs.l);
        assert_eq!(inputs.pk1_agg.limbs.len(), configs.l);
    }
}
