// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Computation types for the pk-bfv circuit: constants, bounds, bit widths, and witness.
//!
//! [`Constants`], [`Bounds`], [`Bits`], and [`Witness`] are produced from BFV parameters
//! and (for witness) a public key. They implement [`Computation`] and are used by codegen.

use crate::circuits::pk_bfv::circuit::PkBfvCircuitInput;
use crate::computation::CircuitComputation;
use crate::computation::Computation;
use crate::computation::ConvertToJson;
use crate::computation::ReduceToZkpModulus;
use crate::errors::CircuitsErrors;
use crate::utils::calculate_bit_width;
use crate::utils::get_zkp_modulus;
use crate::PkBfvCircuit;
use e3_polynomial::center_coefficients_mut;
use e3_polynomial::reduce_coefficients_2d;
use fhe::bfv::BfvParameters;
use fhe_math::rq::Representation;
use itertools::izip;
use num_bigint::BigInt;
use num_bigint::BigUint;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// Output of [`CircuitComputation::compute`] for [`PkBfvCircuit`]: bounds, bit widths, and witness.
#[derive(Debug)]
pub struct PkBfvComputationOutput {
    /// Coefficient bounds for public key polynomials.
    pub bounds: Bounds,
    /// Bit widths for the prover (e.g. pk_bit).
    pub bits: Bits,
    /// Witness data (pk0is, pk1is) for the Noir prover.
    pub witness: Witness,
}

/// Implementation of [`CircuitComputation`] for [`PkBfvCircuit`].
impl CircuitComputation for PkBfvCircuit {
    type Params = BfvParameters;
    type Input = PkBfvCircuitInput;
    type Output = PkBfvComputationOutput;
    type Error = CircuitsErrors;

    fn compute(
        &self,
        params: &Self::Params,
        input: &Self::Input,
    ) -> Result<Self::Output, Self::Error> {
        let bounds = Bounds::compute(params, &())?;
        let bits = Bits::compute(params, &bounds)?;
        let witness = Witness::compute(params, input)?;

        Ok(PkBfvComputationOutput {
            bounds,
            bits,
            witness,
        })
    }
}

/// BFV parameters extracted for the circuit: degree, number of moduli, and modulus values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configs {
    /// Polynomial degree (N).
    pub n: usize,
    /// Number of CRT moduli (L).
    pub l: usize,
    /// CRT moduli q_i.
    pub moduli: Vec<u64>,
    /// Bits.
    pub bits: Bits,
    /// Bounds.
    pub bounds: Bounds,
}

/// Bit widths used by the Noir prover (e.g. for packing coefficients).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bits {
    /// Bit width for public key coefficients.
    pub pk_bit: u32,
}

/// Coefficient bounds for public key polynomials (used to derive bit widths).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bounds {
    /// Bound for public key polynomials (pk0, pk1).
    pub pk_bound: BigUint,
}

/// Witness data for the pk-bfv circuit: public key polynomials in CRT form for the prover.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Witness {
    /// First component of the public key per modulus (coefficients as BigInt).
    pub pk0is: Vec<Vec<BigInt>>,
    /// Second component of the public key per modulus (coefficients as BigInt).
    pub pk1is: Vec<Vec<BigInt>>,
}

impl Computation for Configs {
    type Params = BfvParameters;
    type Input = ();
    type Error = CircuitsErrors;

    fn compute(params: &Self::Params, _: &Self::Input) -> Result<Self, CircuitsErrors> {
        let moduli = params.moduli().to_vec();
        let bounds = Bounds::compute(&params, &())?;
        let bits = Bits::compute(&params, &bounds)?;

        Ok(Configs {
            n: params.degree(),
            l: moduli.len(),
            moduli,
            bits,
            bounds,
        })
    }
}

impl Computation for Bits {
    type Params = BfvParameters;
    type Input = Bounds;
    type Error = crate::utils::ZkHelpersUtilsError;

    fn compute(_: &Self::Params, input: &Self::Input) -> Result<Self, Self::Error> {
        Ok(Bits {
            pk_bit: calculate_bit_width(&input.pk_bound.to_string())?,
        })
    }
}

impl Computation for Bounds {
    type Params = BfvParameters;
    type Input = ();
    type Error = fhe::Error;

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
    type Input = PkBfvCircuitInput;
    type Error = fhe::Error;

    fn compute(params: &Self::Params, input: &Self::Input) -> Result<Self, Self::Error> {
        let moduli = params.moduli();

        // Extract public key components (pk0, pk1) from the ciphertext structure
        // and change representation to Power Basis.
        let mut pk0 = input.public_key.c.c[0].clone();
        let mut pk1 = input.public_key.c.c[1].clone();
        pk0.change_representation(Representation::PowerBasis);
        pk1.change_representation(Representation::PowerBasis);

        let pk0_coeffs = pk0.coefficients();
        let pk1_coeffs = pk1.coefficients();
        let pk0_rows = pk0_coeffs.rows();
        let pk1_rows = pk1_coeffs.rows();

        // Extract and convert public key polynomials per modulus
        // Collect into Vec first to preserve moduli ordering with par_iter
        let zipped: Vec<_> = izip!(moduli, pk0_rows, pk1_rows).collect();
        let results: Vec<(Vec<BigInt>, Vec<BigInt>)> = zipped
            .par_iter()
            .map(|(qi, pk0_coeffs, pk1_coeffs)| {
                let mut pk0i: Vec<BigInt> =
                    pk0_coeffs.iter().rev().map(|&x| BigInt::from(x)).collect();
                let mut pk1i: Vec<BigInt> =
                    pk1_coeffs.iter().rev().map(|&x| BigInt::from(x)).collect();

                center_coefficients_mut(&mut pk0i, &BigInt::from(**qi));
                center_coefficients_mut(&mut pk1i, &BigInt::from(**qi));

                (pk0i, pk1i)
            })
            .collect();

        let (pk0is, pk1is): (Vec<_>, Vec<_>) = results.into_iter().unzip();

        Ok(Witness { pk0is, pk1is })
    }
}

impl ConvertToJson for Configs {
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
    use crate::computation::ConvertToJson;
    use crate::computation::ReduceToZkpModulus;
    use crate::sample::Sample;
    use e3_fhe_params::BfvParamSet;
    use e3_fhe_params::DEFAULT_BFV_PRESET;

    #[test]
    fn test_bound_and_bits_computation_consistency() {
        let params = BfvParamSet::from(DEFAULT_BFV_PRESET).build_arc();
        let bounds = Bounds::compute(&params, &()).unwrap();
        let bits = Bits::compute(&params, &bounds).unwrap();
        let expected_bits = calculate_bit_width(&bounds.pk_bound.to_string()).unwrap();

        assert_eq!(bounds.pk_bound, BigUint::from(34359701504u64));
        assert_eq!(bits.pk_bit, expected_bits);
    }

    #[test]
    fn test_witness_reduction_and_json_roundtrip() {
        let params = BfvParamSet::from(DEFAULT_BFV_PRESET).build_arc();
        let encryption_data = Sample::generate(&params);
        let witness = Witness::compute(
            &params,
            &PkBfvCircuitInput {
                public_key: encryption_data.public_key,
            },
        )
        .unwrap();
        let zkp_reduced = witness.reduce_to_zkp_modulus();
        let json = zkp_reduced.convert_to_json().unwrap();
        let decoded: Witness = serde_json::from_value(json.clone()).unwrap();

        assert_eq!(decoded.pk0is, zkp_reduced.pk0is);
        assert_eq!(decoded.pk1is, zkp_reduced.pk1is);
    }

    #[test]
    fn test_constants_json_roundtrip() {
        let params = BfvParamSet::from(DEFAULT_BFV_PRESET).build_arc();
        let constants = Configs::compute(&params, &()).unwrap();

        let json = constants.convert_to_json().unwrap();
        let decoded: Configs = serde_json::from_value(json).unwrap();

        assert_eq!(decoded.n, constants.n);
        assert_eq!(decoded.l, constants.l);
        assert_eq!(decoded.moduli, constants.moduli);
        assert_eq!(decoded.bits, constants.bits);
        assert_eq!(decoded.bounds, constants.bounds);
    }
}
