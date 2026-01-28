// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::traits::CircuitMeta;
use crate::types::DkgInputType;
use e3_fhe_params::ParameterType;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// Errors produced by the circuit registry.
#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("Unknown circuit: {name}")]
    UnknownCircuit { name: String },
    #[error("Invalid input for circuit {name}: expected {expected}")]
    InvalidInput {
        name: String,
        expected: &'static str,
    },
}

/// Registry for PVSS circuits.
pub struct CircuitRegistry {
    circuits: HashMap<String, Arc<dyn CircuitMeta>>,
}

impl CircuitRegistry {
    /// Build a registry with all known circuits registered.
    pub fn new() -> Self {
        Self {
            circuits: HashMap::new(),
        }
    }

    /// Register a circuit descriptor under a name.
    #[allow(dead_code)]
    fn register(&mut self, circuit: Arc<dyn CircuitMeta>) {
        self.circuits.insert(circuit.name().to_lowercase(), circuit);
    }

    /// Get a circuit descriptor from the registry.
    pub fn get(&self, name: &str) -> Result<Arc<dyn CircuitMeta>, RegistryError> {
        self.circuits
            .get(&name.to_lowercase())
            .cloned()
            .ok_or_else(|| RegistryError::UnknownCircuit {
                name: name.to_string(),
            })
    }

    /// Return supported parameter types for a circuit.
    pub fn supported_parameter_type(&self, name: &str) -> Result<ParameterType, RegistryError> {
        Ok(self.get(name)?.supported_parameter())
    }

    /// Return DKG input type for a circuit, if any.
    pub fn dkg_input_type(&self, name: &str) -> Result<Option<DkgInputType>, RegistryError> {
        Ok(self.get(name)?.dkg_input_type())
    }

    /// Get number of proofs for a circuit.
    /// This is used for determine the number of proofs required for aggregation.
    pub fn n_proofs(&self, name: &str) -> Result<usize, RegistryError> {
        Ok(self.get(name)?.n_proofs())
    }

    /// Get number of public inputs for a circuit.
    /// This is used for determine the number of public inputs required for aggregation.
    pub fn n_public_inputs(&self, name: &str) -> Result<usize, RegistryError> {
        Ok(self.get(name)?.n_public_inputs())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pk_bfv::PkBfvCircuit;
    use crate::traits::Circuit;

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
        let mut registry = CircuitRegistry::new();
        registry.register(Arc::new(PkBfvCircuit));
        let circuit = registry.get(<PkBfvCircuit as Circuit>::NAME).unwrap();

        assert_eq!(circuit.name(), <PkBfvCircuit as Circuit>::NAME);
        assert_eq!(circuit.supported_parameter(), ParameterType::DKG);
        assert!(circuit.dkg_input_type().is_none());
        assert_eq!(circuit.n_proofs(), <PkBfvCircuit as Circuit>::N_PROOFS);
        assert_eq!(
            circuit.n_public_inputs(),
            <PkBfvCircuit as Circuit>::N_PUBLIC_INPUTS
        );
    }
}
