// SPDX-License-Identifier: LGPL-3.0-only
//
// Public key BFV proof generation

use crate::error::NoirProverError;
use crate::prover::NoirProver;
use crate::witness::{CompiledCircuit, WitnessGenerator};
use acir::FieldElement;
use fhe::bfv::{BfvParameters, PublicKey};
use noirc_abi::input_parser::InputValue;
use num_bigint::BigInt;
use std::sync::Arc;
use zkfhe_pkbfv::vectors::PkBfvVectors;

const PK_BFV_CIRCUIT_NAME: &str = "pk_bfv";

/// Result of pk_bfv proof generation
pub struct PkBfvProofResult {
    /// The generated proof
    pub proof: Vec<u8>,
    /// The commitment (public output from the circuit)
    pub commitment: Vec<u8>,
}

/// Generate a proof for the pk_bfv circuit
///
/// This proves knowledge of a valid BFV public key and outputs a commitment.
///
/// # Arguments
/// * `prover` - NoirProver instance
/// * `public_key` - The BFV public key to commit to
/// * `params` - BFV parameters
/// * `e3_id` - Unique identifier for this job (used for temp files)
///
/// # Returns
/// * `PkBfvProofResult` containing the proof and the commitment (public output)
pub async fn prove_pk_bfv(
    prover: &NoirProver,
    public_key: &PublicKey,
    params: &Arc<BfvParameters>,
    e3_id: &str,
) -> Result<PkBfvProofResult, NoirProverError> {
    // 1. Compute the vectors from the public key
    let vectors = PkBfvVectors::compute(public_key, params).map_err(|e| {
        NoirProverError::WitnessGenerationFailed(format!("PkBfvVectors::compute: {}", e))
    })?;

    // 2. Convert to standard form (reduce to zkp modulus)
    let std_vectors = vectors.standard_form();

    // 3. Load the compiled circuit
    let circuit_path = prover
        .circuits_dir()
        .join(format!("{}.json", PK_BFV_CIRCUIT_NAME));
    let circuit = CompiledCircuit::from_file(&circuit_path)?;

    // 4. Build inputs from vectors (all private)
    let inputs = build_pk_bfv_inputs(&std_vectors)?;

    // 5. Generate witness
    let witness_gen = WitnessGenerator::new();
    let witness = witness_gen.generate_witness(&circuit, inputs)?;

    // 6. Generate proof
    let proof = prover
        .generate_proof(PK_BFV_CIRCUIT_NAME, &witness, e3_id)
        .await?;

    // 7. Read commitment (public output) from bb's output
    let commitment_path = prover
        .work_dir()
        .join(e3_id)
        .join("out")
        .join("public_inputs");
    let commitment = tokio::fs::read(&commitment_path).await?;

    Ok(PkBfvProofResult { proof, commitment })
}

/// Verify a pk_bfv proof
///
/// # Arguments
/// * `prover` - NoirProver instance
/// * `proof` - The proof bytes
/// * `commitment` - The commitment (public output)
/// * `e3_id` - Unique identifier for this verification job
pub async fn verify_pk_bfv(
    prover: &NoirProver,
    proof: &[u8],
    commitment: &[u8],
    e3_id: &str,
) -> Result<bool, NoirProverError> {
    // Write commitment to the expected location for bb verify -i
    let job_dir = prover.work_dir().join(e3_id);
    tokio::fs::create_dir_all(&job_dir).await?;

    let out_dir = job_dir.join("out");
    tokio::fs::create_dir_all(&out_dir).await?;

    let commitment_path = out_dir.join("public_inputs");
    tokio::fs::write(&commitment_path, commitment).await?;

    prover.verify_proof(PK_BFV_CIRCUIT_NAME, proof, e3_id).await
}

/// Build InputMap from PkBfvVectors (all private inputs)
fn build_pk_bfv_inputs(vectors: &PkBfvVectors) -> Result<noirc_abi::InputMap, NoirProverError> {
    let mut inputs = noirc_abi::InputMap::new();

    // Convert 2D Vec<Vec<BigInt>> to InputValue
    // The circuit expects: pk0is: [Polynomial { coefficients: [Field] }, ...]
    inputs.insert(
        "pk0is".to_string(),
        bigint_2d_to_polynomial_array(&vectors.pk0is)?,
    );
    inputs.insert(
        "pk1is".to_string(),
        bigint_2d_to_polynomial_array(&vectors.pk1is)?,
    );

    Ok(inputs)
}

/// Convert 2D BigInt vector to array of Polynomial structs
/// Each inner vec becomes a Polynomial { coefficients: [...] }
fn bigint_2d_to_polynomial_array(vecs: &[Vec<BigInt>]) -> Result<InputValue, NoirProverError> {
    let polynomials: Vec<InputValue> = vecs
        .iter()
        .map(|coeffs| {
            // Convert coefficients to Field values
            let field_coeffs: Vec<InputValue> = coeffs
                .iter()
                .map(|b| {
                    let s = b.to_string();
                    let field = FieldElement::try_from_str(&s).unwrap_or_default();
                    InputValue::Field(field)
                })
                .collect();

            // Create struct with "coefficients" field
            let mut struct_fields = std::collections::BTreeMap::new();
            struct_fields.insert("coefficients".to_string(), InputValue::Vec(field_coeffs));

            InputValue::Struct(struct_fields)
        })
        .collect();

    Ok(InputValue::Vec(polynomials))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bigint_2d_to_polynomial_array() {
        let vecs = vec![
            vec![BigInt::from(1), BigInt::from(2)],
            vec![BigInt::from(3), BigInt::from(4)],
        ];

        let result = bigint_2d_to_polynomial_array(&vecs).unwrap();

        match result {
            InputValue::Vec(polynomials) => {
                assert_eq!(polynomials.len(), 2);
                // Each element should be a Struct with "coefficients" field
                match &polynomials[0] {
                    InputValue::Struct(fields) => {
                        assert!(fields.contains_key("coefficients"));
                    }
                    _ => panic!("Expected Struct"),
                }
            }
            _ => panic!("Expected Vec"),
        }
    }
}
