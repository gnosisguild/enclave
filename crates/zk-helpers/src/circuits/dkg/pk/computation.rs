// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Computation types for the pk-bfv circuit: constants, bounds, bit widths, and witness.
//!
//! [`Constants`], [`Bounds`], [`Bits`], and [`Witness`] are produced from BFV parameters
//! and (for witness) a public key. They implement [`Computation`] and are used by codegen.

use crate::calculate_bit_width;
use crate::dkg::pk::PkCircuitInput;
use crate::get_zkp_modulus;
use crate::CircuitsErrors;
use crate::ConvertToJson;
use crate::PkCircuit;
use crate::{CircuitComputation, Computation};
use e3_polynomial::CrtPolynomial;
use fhe::bfv::BfvParameters;
use num_bigint::BigUint;
use serde::{Deserialize, Serialize};

/// Output of [`CircuitComputation::compute`] for [`PkCircuit`]: bounds, bit widths, and witness.
#[derive(Debug)]
pub struct PkComputationOutput {
    pub bounds: Bounds,
    pub bits: Bits,
    pub witness: Witness,
}

/// Implementation of [`CircuitComputation`] for [`PkCircuit`].
impl CircuitComputation for PkCircuit {
    type Params = BfvParameters;
    type Input = PkCircuitInput;
    type Output = PkComputationOutput;
    type Error = CircuitsErrors;

    fn compute(params: &Self::Params, input: &Self::Input) -> Result<Self::Output, Self::Error> {
        let bounds = Bounds::compute(params, &())?;
        let bits = Bits::compute(params, &bounds)?;
        let witness = Witness::compute(params, input)?;

        Ok(PkComputationOutput {
            bounds,
            bits,
            witness,
        })
    }
}

/// BFV parameters extracted for the circuit: degree, number of moduli, and modulus values.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configs {
    pub n: usize,
    pub l: usize,
    pub moduli: Vec<u64>,
    pub bits: Bits,
    pub bounds: Bounds,
}

/// Bit widths used by the Noir prover (e.g. for packing coefficients).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bits {
    pub pk_bit: u32,
}

/// Coefficient bounds for public key polynomials (used to derive bit widths).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bounds {
    pub pk_bound: BigUint,
}

/// Witness data for the pk-bfv circuit: public key polynomials in CRT form for the prover.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Witness {
    /// Public key polynomials (pk0, pk1) for each CRT basis.
    pub pk0is: CrtPolynomial,
    pub pk1is: CrtPolynomial,
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
    type Input = PkCircuitInput;
    type Error = CircuitsErrors;

    fn compute(params: &Self::Params, input: &Self::Input) -> Result<Self, Self::Error> {
        let moduli = params.moduli();

        let mut pk0is = CrtPolynomial::from_fhe_polynomial(&input.public_key.c.c[0]);
        let mut pk1is = CrtPolynomial::from_fhe_polynomial(&input.public_key.c.c[1]);

        pk0is.reverse();
        pk1is.reverse();

        pk0is.center(&moduli)?;
        pk1is.center(&moduli)?;

        let zkp_modulus = &get_zkp_modulus();

        pk0is.reduce_uniform(zkp_modulus);
        pk1is.reduce_uniform(zkp_modulus);

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

#[cfg(test)]
mod tests {
    use super::*;

    use crate::ciphernodes_committee::CiphernodesCommitteeSize;
    use crate::sample::prepare_sample_for_test;
    use crate::ConvertToJson;
    use e3_fhe_params::build_pair_for_preset;
    use e3_fhe_params::BfvPreset;
    use e3_fhe_params::DEFAULT_BFV_PRESET;

    #[test]
    fn test_bound_and_bits_computation_consistency() {
        let (_, dkg_params) = build_pair_for_preset(DEFAULT_BFV_PRESET).unwrap();
        let bounds = Bounds::compute(&dkg_params, &()).unwrap();
        let bits = Bits::compute(&dkg_params, &bounds).unwrap();
        let expected_bits = calculate_bit_width(&bounds.pk_bound.to_string()).unwrap();

        assert_eq!(bounds.pk_bound, BigUint::from(1125899906777088u128));
        assert_eq!(bits.pk_bit, expected_bits);
    }

    #[test]
    fn test_witness_reduction_and_json_roundtrip() {
        let (_, dkg_params) = build_pair_for_preset(DEFAULT_BFV_PRESET).unwrap();
        let sample = prepare_sample_for_test(
            BfvPreset::InsecureThreshold512,
            CiphernodesCommitteeSize::Small,
            None,
        )
        .unwrap();
        let witness = Witness::compute(
            &dkg_params,
            &PkCircuitInput {
                public_key: sample.dkg_public_key,
            },
        )
        .unwrap();
        let json = witness.convert_to_json().unwrap();
        let decoded: Witness = serde_json::from_value(json.clone()).unwrap();

        assert_eq!(decoded.pk0is, witness.pk0is);
        assert_eq!(decoded.pk1is, witness.pk1is);
    }

    #[test]
    fn test_constants_json_roundtrip() {
        let (_, dkg_params) = build_pair_for_preset(DEFAULT_BFV_PRESET).unwrap();
        let constants = Configs::compute(&dkg_params, &()).unwrap();

        let json = constants.convert_to_json().unwrap();
        let decoded: Configs = serde_json::from_value(json).unwrap();

        assert_eq!(decoded.n, constants.n);
        assert_eq!(decoded.l, constants.l);
        assert_eq!(decoded.moduli, constants.moduli);
        assert_eq!(decoded.bits, constants.bits);
        assert_eq!(decoded.bounds, constants.bounds);
    }
}
