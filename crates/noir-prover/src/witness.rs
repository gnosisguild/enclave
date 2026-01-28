// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

// Native witness generation using nargo (following mopro/noir-rs pattern)

use crate::error::NoirProverError;
use acir::{
    circuit::Program,
    native_types::{WitnessMap, WitnessStack},
    FieldElement,
};
use base64::engine::{general_purpose, Engine};
use bn254_blackbox_solver::Bn254BlackBoxSolver;
use nargo::foreign_calls::default::DefaultForeignCallBuilder;
use nargo::ops::execute_program;
use noirc_abi::{input_parser::InputValue, Abi, InputMap};
use serde::{Deserialize, Serialize};

/// Compiled Noir circuit artifact (the JSON from target/<circuit>.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledCircuit {
    /// Base64-encoded, gzipped ACIR bytecode
    pub bytecode: String,
    /// Circuit ABI - maps parameter names to witness indices
    pub abi: Abi,
}

impl CompiledCircuit {
    pub fn from_json(json: &str) -> Result<Self, NoirProverError> {
        serde_json::from_str(json).map_err(NoirProverError::JsonError)
    }

    pub async fn from_file(path: &std::path::Path) -> Result<Self, NoirProverError> {
        let contents = tokio::fs::read_to_string(path).await?;
        Self::from_json(&contents)
    }
}

/// Get the ACIR buffer (compressed) from base64-encoded bytecode
fn get_acir_buffer(circuit_bytecode: &str) -> Result<Vec<u8>, NoirProverError> {
    general_purpose::STANDARD
        .decode(circuit_bytecode)
        .map_err(|e| NoirProverError::SerializationError(format!("Base64 decode: {}", e)))
}

/// Get the Program from circuit bytecode
/// Note: Program::deserialize_program handles gzip decompression internally
fn get_program(circuit_bytecode: &str) -> Result<Program<FieldElement>, NoirProverError> {
    let acir_buffer = get_acir_buffer(circuit_bytecode)?;
    Program::deserialize_program(&acir_buffer)
        .map_err(|e| NoirProverError::SerializationError(format!("ACIR decode: {:?}", e)))
}

/// Execute the circuit and return the solved witness stack
fn execute(
    circuit_bytecode: &str,
    initial_witness: WitnessMap<FieldElement>,
) -> Result<WitnessStack<FieldElement>, NoirProverError> {
    let program = get_program(circuit_bytecode)?;
    let blackbox_solver = Bn254BlackBoxSolver::default();
    let mut foreign_call_executor = DefaultForeignCallBuilder::default().build();

    execute_program(
        &program,
        initial_witness,
        &blackbox_solver,
        &mut foreign_call_executor,
    )
    .map_err(|e| NoirProverError::WitnessGenerationFailed(e.to_string()))
}

/// Serialize witness stack using bincode + gzip (bb expects gzip)
fn serialize_witness(
    witness_stack: &WitnessStack<FieldElement>,
) -> Result<Vec<u8>, NoirProverError> {
    use flate2::write::GzEncoder;
    use flate2::Compression;
    use std::io::Write;

    let buf = bincode::serialize(witness_stack)
        .map_err(|e| NoirProverError::SerializationError(format!("Bincode: {}", e)))?;

    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(&buf)
        .map_err(|e| NoirProverError::SerializationError(format!("Gzip: {}", e)))?;
    encoder
        .finish()
        .map_err(|e| NoirProverError::SerializationError(format!("Gzip finish: {}", e)))
}

/// Native witness generator
pub struct WitnessGenerator;

impl WitnessGenerator {
    pub fn new() -> Self {
        Self
    }

    /// Generate witness from a compiled circuit and inputs
    ///
    /// Returns serialized witness data ready for `bb prove`
    pub async fn generate_witness(
        &self,
        circuit: &CompiledCircuit,
        inputs: InputMap,
    ) -> Result<Vec<u8>, NoirProverError> {
        let bytecode = circuit.bytecode.clone();
        let abi = circuit.abi.clone();

        tokio::task::spawn_blocking(move || {
            let initial_witness = abi.encode(&inputs, None).map_err(|e| {
                NoirProverError::WitnessGenerationFailed(format!("ABI encode: {:?}", e))
            })?;

            let witness_stack = execute(&bytecode, initial_witness)?;
            serialize_witness(&witness_stack)
        })
        .await
        .map_err(|e| NoirProverError::WitnessGenerationFailed(e.to_string()))?
    }
}

impl Default for WitnessGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper to create InputMap from string key-value pairs
pub fn input_map<I, K, V>(iter: I) -> InputMap
where
    I: IntoIterator<Item = (K, V)>,
    K: Into<String>,
    V: AsRef<str>,
{
    iter.into_iter()
        .map(|(k, v)| {
            let v_str = v.as_ref();
            let field = FieldElement::try_from_str(v_str).unwrap_or_default();
            (k.into(), InputValue::Field(field))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const DUMMY_CIRCUIT: &str = r#"{"noir_version":"1.0.0-beta.15+83245db91dcf63420ef4bcbbd85b98f397fee663","hash":"15412581843239610929","abi":{"parameters":[{"name":"x","type":{"kind":"field"},"visibility":"private"},{"name":"y","type":{"kind":"field"},"visibility":"private"},{"name":"_sum","type":{"kind":"field"},"visibility":"public"}],"return_type":null,"error_types":{}},"bytecode":"H4sIAAAAAAAA/5WOMQ5AMBRA/y8HMbIRRxCJSYwWg8RiIGIz9gjiAk4hHKeb0WLX0KHRDu1bXvL/y89H+HCFu7rtCTeCiiPsgRFo06LUhk0+smgN9iLdKC0rPz6z6RjmhN3LxffE/O7byg+hZv7nAb2HRPkUAQAA","debug_symbols":"jZDRCoMwDEX/Jc996MbG1F8ZQ2qNUghtie1giP++KLrpw2BPaXJ7bsgdocUm97XzXRiguo/QsCNyfU3BmuSCl+k4KdjaOjGijGCnCxUNo09Q+Uyk4GkoL5+GaPxSk2FRtQL0rVQx7Bzh/JrUl9a/0Vu5ssXlA1//psvbSp90ccAf0hnr+HAuaKjO0+zGzjSEawRd9naXSHrFTdkyixwstplxtls0WfAG","file_map":{"50":{"source":"pub fn main(\n    x: Field,\n    y: Field,\n    _sum: pub Field\n) {\n    let sum = x + y;\n    assert(sum == _sum);\n}\n","path":"/Users/ctrlc03/Documents/zk/enclave/circuits/bin/dummy/src/main.nr"}},"expression_width":{"Bounded":{"width":4}}}"#;

    #[test]
    fn test_load_circuit() {
        let circuit = CompiledCircuit::from_json(DUMMY_CIRCUIT).unwrap();
        assert_eq!(circuit.abi.parameters.len(), 3);
    }

    #[tokio::test]
    async fn test_generate_witness() {
        let circuit = CompiledCircuit::from_json(DUMMY_CIRCUIT).unwrap();
        let generator = WitnessGenerator::new();
        let inputs = input_map([("x", "5"), ("y", "3"), ("_sum", "8")]);

        let witness = generator.generate_witness(&circuit, inputs).await.unwrap();

        assert!(witness.len() > 2);
        assert_eq!(witness[0], 0x1f);
        assert_eq!(witness[1], 0x8b);
    }

    #[tokio::test]
    async fn test_wrong_sum_fails() {
        let circuit = CompiledCircuit::from_json(DUMMY_CIRCUIT).unwrap();
        let generator = WitnessGenerator::new();
        let inputs = input_map([("x", "5"), ("y", "3"), ("_sum", "10")]);

        let result = generator.generate_witness(&circuit, inputs).await;

        assert!(result.is_err());
    }
}
