// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_polynomial::reduce_coefficients_2d;
use e3_polynomial::utils::reduce_and_center_coefficients_mut;
use e3_zk_helpers::utils::calculate_bit_width;
use e3_zk_helpers::utils::get_zkp_modulus;
use e3_zk_helpers::utils::Result as ZkUtilsResult;
use fhe::bfv::BfvParameters;
use fhe::bfv::PublicKey;
use fhe_math::rq::Representation;
use itertools::izip;
use num_bigint::BigInt;
use num_bigint::BigUint;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

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

impl Bits {
    pub fn compute(bfv_params: &BfvParameters) -> ZkUtilsResult<Bits> {
        let pk_bound = Bounds::compute(bfv_params);

        Ok(Bits {
            pk_bit: calculate_bit_width(&pk_bound.pk_bound.to_string())?,
        })
    }
}

impl Bounds {
    pub fn compute(bfv_params: &BfvParameters) -> Bounds {
        let mut pk_bound_max = BigUint::from(0u32);

        for &qi in bfv_params.moduli() {
            let qi_bound: BigUint = (&BigUint::from(qi) - 1u32) / 2u32;

            if qi_bound > pk_bound_max {
                pk_bound_max = qi_bound;
            }
        }

        Bounds {
            pk_bound: pk_bound_max,
        }
    }
}

impl Witness {
    pub fn compute(
        bfv_params: &BfvParameters,
        public_key: &PublicKey,
    ) -> Result<Witness, fhe::Error> {
        let moduli = bfv_params.moduli();

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

    pub fn to_zkp_modulus(&self) -> Witness {
        Witness {
            pk0is: reduce_coefficients_2d(&self.pk0is, &get_zkp_modulus()),
            pk1is: reduce_coefficients_2d(&self.pk1is, &get_zkp_modulus()),
        }
    }

    pub fn to_json(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample::generate_sample;
    use e3_fhe_params::{BfvParamSet, BfvPreset};

    #[test]
    fn test_bound_and_bits_computation() {
        let bfv_params = BfvParamSet::from(BfvPreset::InsecureThresholdBfv512).build_arc();
        let bounds = Bounds::compute(&bfv_params);
        let bits = Bits::compute(&bfv_params).unwrap();

        assert_eq!(bounds.pk_bound, BigUint::from(34359701504u64));
        assert_eq!(bits.pk_bit, 35);
    }

    #[test]
    fn test_witness_computation_and_conversion() {
        let bfv_params = BfvParamSet::from(BfvPreset::InsecureThresholdBfv512).build_arc();

        let encryption_data = generate_sample(&bfv_params);

        let witness = Witness::compute(&bfv_params, &encryption_data.public_key).unwrap();

        let zkp_reduced = witness.to_zkp_modulus();
        let json = zkp_reduced.to_json().unwrap();
        let decoded: Witness = serde_json::from_value(json.clone()).unwrap();

        assert_eq!(decoded.pk0is, zkp_reduced.pk0is);
        assert_eq!(decoded.pk1is, zkp_reduced.pk1is);
    }
}
