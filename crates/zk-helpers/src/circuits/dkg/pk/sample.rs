// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Sample data generation for the pk circuit: committee and DKG public key only.

use crate::ciphernodes_committee::CiphernodesCommittee;
use crate::ciphernodes_committee::CiphernodesCommitteeSize;
use crate::CircuitsErrors;
use e3_fhe_params::build_pair_for_preset;
use e3_fhe_params::BfvPreset;
use fhe::bfv::{BfvParameters, PublicKey, SecretKey};
use rand::thread_rng;
use std::sync::Arc;

/// Sample data for the **pk** circuit: committee and DKG public key only.
#[derive(Debug, Clone)]
pub struct PkSample {
    /// Committee information.
    pub committee: CiphernodesCommittee,
    /// DKG BFV public key.
    pub dkg_public_key: PublicKey,
}

impl PkSample {
    /// Generates sample data for the pk circuit.
    pub fn generate(
        _threshold_params: &Arc<BfvParameters>,
        dkg_params: &Arc<BfvParameters>,
        committee_size: CiphernodesCommitteeSize,
    ) -> Result<Self, CircuitsErrors> {
        let mut rng = thread_rng();
        let committee = committee_size.values();
        let dkg_secret_key = SecretKey::random(dkg_params, &mut rng);
        let dkg_public_key = PublicKey::new(&dkg_secret_key, &mut rng);
        Ok(Self {
            committee,
            dkg_public_key,
        })
    }
}

/// Prepares a pk sample for testing using a threshold preset (DKG params come from its pair).
pub fn prepare_pk_sample_for_test(
    threshold_preset: BfvPreset,
    committee: CiphernodesCommitteeSize,
) -> Result<PkSample, CircuitsErrors> {
    let (threshold_params, dkg_params) = build_pair_for_preset(threshold_preset)
        .map_err(|e| CircuitsErrors::Sample(e.to_string()))?;
    PkSample::generate(&threshold_params, &dkg_params, committee)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ciphernodes_committee::CiphernodesCommitteeSize;
    use e3_fhe_params::BfvPreset;

    #[test]
    fn test_generate_pk_sample() {
        let committee = CiphernodesCommitteeSize::Small.values();
        let sample = prepare_pk_sample_for_test(
            BfvPreset::InsecureThreshold512,
            CiphernodesCommitteeSize::Small,
        )
        .unwrap();

        assert_eq!(sample.committee.n, committee.n);
        assert_eq!(sample.committee.threshold, committee.threshold);
        assert_eq!(sample.committee.h, committee.h);
        assert_eq!(sample.dkg_public_key.c.c.len(), 2);
    }
}
