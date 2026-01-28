// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::traits::Computation;
use crate::traits::ConvertToJson;
use crate::traits::ReduceToZkpModulus;
use e3_polynomial::reduce_coefficients_2d;
use e3_polynomial::utils::reduce_and_center_coefficients_mut;
use e3_zk_helpers::utils::calculate_bit_width;
use e3_zk_helpers::utils::get_zkp_modulus;
use fhe::bfv::BfvParameters;
use fhe::bfv::PublicKey;
use fhe_math::rq::Representation;
use itertools::izip;
use num_bigint::BigInt;
use num_bigint::BigUint;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constants {
    pub n: usize,
    pub l: usize,
    pub moduli: Vec<u64>,
}

#[derive(Debug, Clone)]
pub struct Bits {
    pub pk_bit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bounds {
    /// Bound for public key polynomials (pk0, pk1)
    pub pk_bound: BigUint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Witness {
    /// Public key polynomials (pk0, pk1) for each CRT basis.
    pub pk0is: Vec<Vec<BigInt>>,
    pub pk1is: Vec<Vec<BigInt>>,
}

impl Computation for Constants {
    type Params = BfvParameters;
    type Input = ();
    type Error = std::convert::Infallible;

    fn compute(params: &Self::Params, _: &Self::Input) -> Result<Self, Self::Error> {
        let moduli = params.moduli().to_vec();

        Ok(Constants {
            n: params.degree(),
            l: moduli.len(),
            moduli,
        })
    }
}

impl Computation for Bits {
    type Params = BfvParameters;
    type Input = Bounds;
    type Error = e3_zk_helpers::utils::ZkHelpersUtilsError;

    fn compute(_: &Self::Params, input: &Self::Input) -> Result<Self, Self::Error> {
        Ok(Bits {
            pk_bit: calculate_bit_width(&input.pk_bound.to_string())?,
        })
    }
}

impl Computation for Bounds {
    type Params = BfvParameters;
    type Input = ();
    type Error = crate::errors::CodegenError;

    fn compute(params: &Self::Params, _: &Self::Input) -> Result<Self, Self::Error> {
        let mut pk_bound_max = BigUint::from(0u32);

        for &qi in params.moduli() {
            let qi_bound: BigUint = (&BigUint::from(qi) - 1u32) / 2u32;

            if qi_bound > pk_bound_max {
                pk_bound_max = qi_bound;
            }
        }

        Ok(Bounds {
            pk_bound: pk_bound_max,
        })
    }
}

impl Computation for Witness {
    type Params = BfvParameters;
    type Input = PublicKey;
    type Error = fhe::Error;

    fn compute(params: &Self::Params, public_key: &Self::Input) -> Result<Self, Self::Error> {
        let moduli = params.moduli();

        // Extract public key components (pk0, pk1) from the ciphertext structure
        // and change representation to Power Basis.
        let mut pk0 = public_key.c.c[0].clone();
        let mut pk1 = public_key.c.c[1].clone();
        pk0.change_representation(Representation::PowerBasis);
        pk1.change_representation(Representation::PowerBasis);

        let pk0_coeffs = pk0.coefficients();
        let pk1_coeffs = pk1.coefficients();
        let pk0_rows = pk0_coeffs.rows();
        let pk1_rows = pk1_coeffs.rows();

        // Extract and convert public key polynomials per modulus
        let results: Vec<(Vec<BigInt>, Vec<BigInt>)> = izip!(moduli, pk0_rows, pk1_rows)
            .par_bridge()
            .map(|(qi, pk0_coeffs, pk1_coeffs)| {
                let mut pk0i: Vec<BigInt> =
                    pk0_coeffs.iter().rev().map(|&x| BigInt::from(x)).collect();
                let mut pk1i: Vec<BigInt> =
                    pk1_coeffs.iter().rev().map(|&x| BigInt::from(x)).collect();

                reduce_and_center_coefficients_mut(&mut pk0i, &BigInt::from(*qi));
                reduce_and_center_coefficients_mut(&mut pk1i, &BigInt::from(*qi));

                (pk0i, pk1i)
            })
            .collect();

        let (pk0is, pk1is): (Vec<_>, Vec<_>) = results.into_iter().unzip();

        Ok(Witness { pk0is, pk1is })
    }
}

impl ConvertToJson for Constants {
    fn convert_to_json(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }
}

impl ConvertToJson for Bounds {
    fn convert_to_json(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }
}

impl ConvertToJson for Witness {
    fn convert_to_json(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }
}

impl ReduceToZkpModulus for Witness {
    fn reduce_to_zkp_modulus(&self) -> Witness {
        Witness {
            pk0is: reduce_coefficients_2d(&self.pk0is, &get_zkp_modulus()),
            pk1is: reduce_coefficients_2d(&self.pk1is, &get_zkp_modulus()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample::generate_sample;
    use crate::traits::ConvertToJson;
    use crate::traits::ReduceToZkpModulus;
    use e3_fhe_params::{BfvParamSet, BfvPreset};

    #[test]
    fn test_bound_and_bits_computation_consistency() {
        let params = BfvParamSet::from(BfvPreset::InsecureThresholdBfv512).build_arc();
        let bounds = Bounds::compute(&params, &()).unwrap();
        let bits = Bits::compute(&params, &bounds).unwrap();
        let expected_bits = calculate_bit_width(&bounds.pk_bound.to_string()).unwrap();

        assert_eq!(bounds.pk_bound, BigUint::from(34359701504u64));
        assert_eq!(bits.pk_bit, expected_bits);
    }

    #[test]
    fn test_witness_reduction_and_json_roundtrip() {
        let params = BfvParamSet::from(BfvPreset::InsecureThresholdBfv512).build_arc();
        let encryption_data = generate_sample(&params);
        let witness = Witness::compute(&params, &encryption_data.public_key).unwrap();
        let zkp_reduced = witness.reduce_to_zkp_modulus();
        let json = zkp_reduced.convert_to_json().unwrap();
        let decoded: Witness = serde_json::from_value(json.clone()).unwrap();

        assert_eq!(decoded.pk0is, zkp_reduced.pk0is);
        assert_eq!(decoded.pk1is, zkp_reduced.pk1is);
    }

    #[test]
    fn test_constants_json_roundtrip() {
        let params = BfvParamSet::from(BfvPreset::InsecureThresholdBfv512).build_arc();
        let constants = Constants::compute(&params, &()).unwrap();

        let json = constants.convert_to_json().unwrap();
        let decoded: Constants = serde_json::from_value(json).unwrap();

        assert_eq!(decoded.n, constants.n);
        assert_eq!(decoded.l, constants.l);
        assert_eq!(decoded.moduli, constants.moduli);
    }
}
