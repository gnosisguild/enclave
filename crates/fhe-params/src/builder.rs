// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::constants::{insecure_512, secure_8192};
use crate::presets::{BfvParamSet, BfvPreset, PresetError};
use fhe::bfv::{BfvParameters, BfvParametersBuilder};
use num_bigint::BigUint;
use std::sync::Arc;

pub fn build_pair_for_preset(
    preset: BfvPreset,
) -> Result<(Arc<BfvParameters>, Arc<BfvParameters>), PresetError> {
    match preset {
        BfvPreset::InsecureThresholdBfv512 => {
            let params_trbfv = BfvParametersBuilder::new()
                .set_degree(insecure_512::DEGREE)
                .set_plaintext_modulus(insecure_512::threshold::PLAINTEXT_MODULUS)
                .set_moduli(insecure_512::threshold::MODULI)
                .set_error1_variance(BigUint::from(
                    insecure_512::threshold::ERROR1_VARIANCE_BIGUINT,
                ))
                .build_arc()
                .unwrap();

            let params_bfv = BfvParametersBuilder::new()
                .set_degree(insecure_512::DEGREE)
                .set_plaintext_modulus(insecure_512::dkg::PLAINTEXT_MODULUS)
                .set_moduli(insecure_512::dkg::MODULI)
                .set_variance(insecure_512::dkg::VARIANCE as usize)
                .build_arc()
                .unwrap();

            Ok((params_trbfv, params_bfv))
        }
        BfvPreset::SecureThresholdBfv8192 => {
            let params_trbfv = BfvParametersBuilder::new()
                .set_degree(secure_8192::DEGREE)
                .set_plaintext_modulus(secure_8192::threshold::PLAINTEXT_MODULUS)
                .set_moduli(secure_8192::threshold::MODULI)
                .set_error1_variance_str(secure_8192::threshold::ERROR1_VARIANCE)
                .unwrap()
                .build_arc()
                .unwrap();

            let params_bfv = BfvParametersBuilder::new()
                .set_degree(secure_8192::DEGREE)
                .set_plaintext_modulus(secure_8192::dkg::BFV_PLAINTEXT_MODULUS)
                .set_moduli(secure_8192::dkg::BFV_MODULI)
                .build_arc()
                .unwrap();

            Ok((params_trbfv, params_bfv))
        }
        other => Err(PresetError::MissingPair(other.name())),
    }
}

pub fn build_bfv_params_from_set(param_set: BfvParamSet) -> BfvParameters {
    build_bfv_params(
        param_set.degree,
        param_set.plaintext_modulus,
        param_set.moduli,
        param_set.error1_variance,
    )
}

pub fn build_bfv_params_from_set_arc(param_set: BfvParamSet) -> Arc<BfvParameters> {
    build_bfv_params_arc(
        param_set.degree,
        param_set.plaintext_modulus,
        param_set.moduli,
        param_set.error1_variance,
    )
}

pub fn build_bfv_params(
    degree: usize,
    plaintext_modulus: u64,
    moduli: &[u64],
    error1_variance: Option<&str>,
) -> BfvParameters {
    let mut builder = BfvParametersBuilder::new();
    builder
        .set_degree(degree)
        .set_plaintext_modulus(plaintext_modulus)
        .set_moduli(moduli);

    if let Some(error1) = error1_variance {
        builder
            .set_error1_variance_str(error1)
            .unwrap_or_else(|e| panic!("Failed to set error1_variance: {}", e));
    }

    builder
        .build()
        .unwrap_or_else(|e| panic!("Failed to build BFV Parameters: {}", e))
}

pub fn build_bfv_params_arc(
    degree: usize,
    plaintext_modulus: u64,
    moduli: &[u64],
    error1_variance: Option<&str>,
) -> Arc<BfvParameters> {
    let mut builder = BfvParametersBuilder::new();
    builder
        .set_degree(degree)
        .set_plaintext_modulus(plaintext_modulus)
        .set_moduli(moduli);

    if let Some(error1) = error1_variance {
        builder
            .set_error1_variance_str(error1)
            .unwrap_or_else(|e| panic!("Failed to set error1_variance: {}", e));
    }

    builder
        .build_arc()
        .unwrap_or_else(|e| panic!("Failed to build BFV Parameters wrapped in Arc: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{defaults, insecure_512, secure_8192};
    use crate::presets::BfvPreset;
    use num_bigint::BigUint;
    use std::str::FromStr;

    #[test]
    fn test_build_bfv_params_insecure_dkg() {
        // Test building BFV params using insecure DKG preset constants
        let degree = insecure_512::DEGREE;
        let plaintext_modulus = insecure_512::dkg::PLAINTEXT_MODULUS;
        let moduli = insecure_512::dkg::MODULI;

        let params = build_bfv_params(degree, plaintext_modulus, moduli, None);
        assert_eq!(params.degree(), degree);
        assert_eq!(params.plaintext(), plaintext_modulus);
        assert_eq!(params.moduli(), moduli);
        assert_eq!(params.variance(), defaults::VARIANCE);
        assert_eq!(
            params.get_error1_variance(),
            &BigUint::from(defaults::ERROR1_VARIANCE)
        );
    }

    #[test]
    fn test_build_bfv_params_arc_insecure_dkg() {
        // Test building Arc<BFV params> using insecure DKG preset constants
        let degree = insecure_512::DEGREE;
        let plaintext_modulus = insecure_512::dkg::PLAINTEXT_MODULUS;
        let moduli = insecure_512::dkg::MODULI;

        let params = build_bfv_params_arc(degree, plaintext_modulus, moduli, None);
        assert_eq!(params.degree(), degree);
        assert_eq!(params.plaintext(), plaintext_modulus);
        assert_eq!(params.moduli(), moduli);
        assert_eq!(params.variance(), defaults::VARIANCE);
        assert_eq!(
            params.get_error1_variance(),
            &BigUint::from(defaults::ERROR1_VARIANCE)
        );
    }

    #[test]
    fn test_build_trbfv_params_secure_threshold() {
        // Test building TRBFV params using secure threshold preset constants
        let degree = secure_8192::DEGREE;
        let plaintext_modulus = secure_8192::threshold::PLAINTEXT_MODULUS;
        let moduli = secure_8192::threshold::MODULI;
        let error1_variance = secure_8192::threshold::ERROR1_VARIANCE;

        let params = build_bfv_params(degree, plaintext_modulus, moduli, Some(error1_variance));
        assert_eq!(params.degree(), degree);
        assert_eq!(params.plaintext(), plaintext_modulus);
        assert_eq!(params.moduli(), moduli);
        assert_eq!(params.variance(), defaults::VARIANCE);
        assert_eq!(
            params.get_error1_variance(),
            &BigUint::from_str(error1_variance).unwrap()
        );
    }

    #[test]
    fn test_build_trbfv_params_arc_secure_threshold() {
        // Test building Arc<TRBFV params> using secure threshold preset constants
        let degree = secure_8192::DEGREE;
        let plaintext_modulus = secure_8192::threshold::PLAINTEXT_MODULUS;
        let moduli = secure_8192::threshold::MODULI;
        let error1_variance = secure_8192::threshold::ERROR1_VARIANCE;

        let params = build_bfv_params_arc(degree, plaintext_modulus, moduli, Some(error1_variance));
        assert_eq!(params.degree(), degree);
        assert_eq!(params.plaintext(), plaintext_modulus);
        assert_eq!(params.moduli(), moduli);
        assert_eq!(params.variance(), defaults::VARIANCE);
        assert_eq!(
            params.get_error1_variance(),
            &BigUint::from_str(error1_variance).unwrap()
        );
    }

    #[test]
    fn test_build_bfv_params_from_set_insecure_dkg() {
        // Test building from BfvParamSet using insecure DKG preset
        let preset = BfvPreset::InsecureDkg512;
        let param_set = preset.into();
        let params = build_bfv_params_from_set(param_set);

        assert_eq!(params.degree(), insecure_512::DEGREE);
        assert_eq!(params.plaintext(), insecure_512::dkg::PLAINTEXT_MODULUS);
        assert_eq!(params.moduli(), insecure_512::dkg::MODULI);
    }

    #[test]
    fn test_build_bfv_params_from_set_arc_insecure_dkg() {
        // Test building Arc from BfvParamSet using insecure DKG preset
        let preset = BfvPreset::InsecureDkg512;
        let param_set = preset.into();
        let params = build_bfv_params_from_set_arc(param_set);

        assert_eq!(params.degree(), insecure_512::DEGREE);
        assert_eq!(params.plaintext(), insecure_512::dkg::PLAINTEXT_MODULUS);
        assert_eq!(params.moduli(), insecure_512::dkg::MODULI);
    }

    #[test]
    fn test_build_bfv_params_from_set_secure_threshold() {
        // Test building from BfvParamSet using secure threshold preset
        let preset = BfvPreset::SecureThresholdBfv8192;
        let param_set = preset.into();
        let params = build_bfv_params_from_set(param_set);

        assert_eq!(params.degree(), secure_8192::DEGREE);
        assert_eq!(
            params.plaintext(),
            secure_8192::threshold::PLAINTEXT_MODULUS
        );
        assert_eq!(params.moduli(), secure_8192::threshold::MODULI);
        assert_eq!(
            params.get_error1_variance(),
            &BigUint::from_str(secure_8192::threshold::ERROR1_VARIANCE).unwrap()
        );
    }

    #[test]
    fn test_build_bfv_params_from_set_arc_secure_threshold() {
        // Test building Arc from BfvParamSet using secure threshold preset
        let preset = BfvPreset::SecureThresholdBfv8192;
        let param_set = preset.into();
        let params = build_bfv_params_from_set_arc(param_set);

        assert_eq!(params.degree(), secure_8192::DEGREE);
        assert_eq!(
            params.plaintext(),
            secure_8192::threshold::PLAINTEXT_MODULUS
        );
        assert_eq!(params.moduli(), secure_8192::threshold::MODULI);
        assert_eq!(
            params.get_error1_variance(),
            &BigUint::from_str(secure_8192::threshold::ERROR1_VARIANCE).unwrap()
        );
    }
}
