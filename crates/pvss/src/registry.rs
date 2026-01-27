// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::circuits::pk_bfv::{PK_BFV_CIRCUIT_NAME, PK_BFV_N_PROOFS, PK_BFV_N_PUBLIC_INPUTS};
use e3_fhe_params::ParameterType;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

type PublicInputsFn = Arc<dyn Fn() -> u32 + Send + Sync + 'static>;

#[derive(Clone)]
pub struct ZKCircuit {
    name: &'static str,
    compute: CircuitComputeMetadata,
    codegen: CircuitCodegenMetadata,
}

#[derive(Clone)]
pub struct CircuitComputeMetadata {
    supported_parameter: ParameterType,
}

#[derive(Clone)]
pub struct CircuitCodegenMetadata {
    n_proofs: u32,
    n_public_inputs: PublicInputsFn,
}

/// Errors produced by the circuit registry.
#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("Unknown circuit: {name}")]
    UnknownCircuit { name: String },
}

/// Registry for PVSS circuits.
pub struct CircuitRegistry {
    circuits: HashMap<String, ZKCircuit>,
}

impl CircuitRegistry {
    /// Build a registry with all known circuits registered.
    pub fn new() -> Self {
        let mut registry = Self {
            circuits: HashMap::new(),
        };

        registry.register(
            PK_BFV_CIRCUIT_NAME,
            CircuitComputeMetadata {
                supported_parameter: ParameterType::DKG,
            },
            CircuitCodegenMetadata {
                n_proofs: PK_BFV_N_PROOFS,
                n_public_inputs: Arc::new(|| PK_BFV_N_PUBLIC_INPUTS),
            },
        );

        registry
    }

    /// Register a circuit descriptor under a name.
    fn register(
        &mut self,
        name: &'static str,
        compute: CircuitComputeMetadata,
        codegen: CircuitCodegenMetadata,
    ) {
        self.circuits.insert(
            name.to_lowercase(),
            ZKCircuit {
                name,
                compute,
                codegen,
            },
        );
    }

    /// Get a circuit descriptor from the registry.
    pub fn get(&self, name: &str) -> Result<ZKCircuit, RegistryError> {
        let key = name.to_lowercase();
        let ZKCircuit {
            name,
            compute,
            codegen,
        } = self
            .circuits
            .get(&key)
            .ok_or_else(|| RegistryError::UnknownCircuit {
                name: name.to_string(),
            })?;
        Ok(ZKCircuit {
            name,
            compute: compute.clone(),
            codegen: codegen.clone(),
        })
    }

    /// Return supported parameter types for a circuit.
    pub fn supported_parameter_type(&self, name: &str) -> Result<ParameterType, RegistryError> {
        let ZKCircuit { compute, .. } =
            self.circuits.get(&name.to_lowercase()).ok_or_else(|| {
                RegistryError::UnknownCircuit {
                    name: name.to_string(),
                }
            })?;
        Ok(compute.supported_parameter)
    }

    /// Get number of proofs for a circuit.
    /// This is used for determine the number of proofs required for aggregation.
    pub fn n_proofs(&self, name: &str) -> Result<u32, RegistryError> {
        let ZKCircuit { codegen, .. } =
            self.circuits.get(&name.to_lowercase()).ok_or_else(|| {
                RegistryError::UnknownCircuit {
                    name: name.to_string(),
                }
            })?;
        Ok(codegen.n_proofs)
    }

    /// Get number of public inputs for a circuit.
    /// This is used for determine the number of public inputs required for aggregation.
    pub fn n_public_inputs(&self, name: &str) -> Result<u32, RegistryError> {
        let ZKCircuit { codegen, .. } =
            self.circuits.get(&name.to_lowercase()).ok_or_else(|| {
                RegistryError::UnknownCircuit {
                    name: name.to_string(),
                }
            })?;
        Ok((codegen.n_public_inputs)())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_rejects_unknown_circuit() {
        let registry = CircuitRegistry::new();
        assert!(matches!(
            registry.get("unknown"),
            Err(RegistryError::UnknownCircuit { .. })
        ));
    }

    #[test]
    fn registry_reports_expected_metadata() {
        let registry = CircuitRegistry::new();
        let ZKCircuit {
            name,
            compute,
            codegen,
        } = registry.get(PK_BFV_CIRCUIT_NAME).unwrap();

        assert_eq!(name, PK_BFV_CIRCUIT_NAME);
        assert_eq!(compute.supported_parameter, ParameterType::DKG);
        assert_eq!(codegen.n_proofs, PK_BFV_N_PROOFS);
        assert_eq!((codegen.n_public_inputs)(), PK_BFV_N_PUBLIC_INPUTS);
    }
}
