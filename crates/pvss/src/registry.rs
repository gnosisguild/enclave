// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use e3_fhe_params::ParameterType;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// Errors produced by the circuit registry.
#[derive(Error, Debug)]
pub enum RegistryError {
    #[error("Unknown circuit: {name}")]
    UnknownCircuit { name: String },
}

/// Variant for input types for DKG.
#[derive(Clone)]
pub enum DkgInputType {
    /// The input type that generates shares of a secret key using secret sharing.
    SecretKey,
    /// The input type that generates shares of smudging noise instead of secret key shares.
    SmudgingNoise,
}

/// Trait for circuit metadata.
pub trait Circuit: Send + Sync {
    const NAME: &'static str;
    const PREFIX: &'static str;
    const SUPPORTED_PARAMETER: ParameterType;
    const DKG_INPUT_TYPE: Option<DkgInputType>;
    const N_PROOFS: usize;
    const N_PUBLIC_INPUTS: usize;

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn prefix(&self) -> &'static str {
        Self::PREFIX
    }

    fn supported_parameter(&self) -> ParameterType {
        Self::SUPPORTED_PARAMETER
    }

    fn dkg_input_type(&self) -> Option<DkgInputType> {
        Self::DKG_INPUT_TYPE
    }

    fn n_recursive_proofs(&self) -> usize {
        Self::N_PROOFS
    }

    fn n_public_inputs(&self) -> usize {
        Self::N_PUBLIC_INPUTS
    }
}

pub trait CircuitMetadata: Send + Sync {
    fn name(&self) -> &'static str;
    fn supported_parameter(&self) -> ParameterType;
    fn dkg_input_type(&self) -> Option<DkgInputType>;
    fn n_recursive_proofs(&self) -> usize;
    fn n_public_inputs(&self) -> usize;
}

impl<T: Circuit> CircuitMetadata for T {
    fn name(&self) -> &'static str {
        T::NAME
    }

    fn supported_parameter(&self) -> ParameterType {
        T::SUPPORTED_PARAMETER
    }

    fn dkg_input_type(&self) -> Option<DkgInputType> {
        T::DKG_INPUT_TYPE
    }

    fn n_recursive_proofs(&self) -> usize {
        T::N_PROOFS
    }

    fn n_public_inputs(&self) -> usize {
        T::N_PUBLIC_INPUTS
    }
}

/// Registry for PVSS circuits.
pub struct CircuitRegistry {
    circuits: HashMap<String, Arc<dyn CircuitMetadata>>,
}

impl CircuitRegistry {
    /// Build an empty registry.
    pub fn new() -> Self {
        Self {
            circuits: HashMap::new(),
        }
    }

    /// Register a circuit descriptor under a name.
    pub fn register(&mut self, circuit: Arc<dyn CircuitMetadata>) {
        self.circuits.insert(circuit.name().to_lowercase(), circuit);
    }

    /// Get a circuit descriptor from the registry.
    pub fn get(&self, name: &str) -> Result<Arc<dyn CircuitMetadata>, RegistryError> {
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

    /// Get number of recursive proofs for a circuit.
    /// This is used for determine the number of proofs required for aggregation.
    pub fn n_recursive_proofs(&self, name: &str) -> Result<usize, RegistryError> {
        Ok(self.get(name)?.n_recursive_proofs())
    }

    /// Get number of public inputs for a circuit.
    /// This is used for determine the number of public inputs required for aggregation.
    pub fn n_public_inputs(&self, name: &str) -> Result<usize, RegistryError> {
        Ok(self.get(name)?.n_public_inputs())
    }

    /// List all registered circuit names.
    pub fn list_circuits(&self) -> Vec<String> {
        self.circuits.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub struct TestCircuit;

    impl Circuit for TestCircuit {
        const NAME: &'static str = "test";
        const PREFIX: &'static str = "TEST";
        const SUPPORTED_PARAMETER: ParameterType = ParameterType::DKG;
        const DKG_INPUT_TYPE: Option<DkgInputType> = Some(DkgInputType::SecretKey);
        const N_PROOFS: usize = 1;
        const N_PUBLIC_INPUTS: usize = 1;
    }

    #[test]
    /// Unknown circuits should return an error.
    fn registry_rejects_unknown_circuit() {
        let registry = CircuitRegistry::new();
        assert!(matches!(
            registry.get("unknown"),
            Err(RegistryError::UnknownCircuit { .. })
        ));
    }

    #[test]
    /// Registry should expose metadata for registered circuits.
    fn registry_reports_expected_metadata() {
        let mut registry = CircuitRegistry::new();
        registry.register(Arc::new(TestCircuit));
        let circuit = registry.get(<TestCircuit as Circuit>::NAME).unwrap();

        assert_eq!(circuit.name(), <TestCircuit as Circuit>::NAME);
        assert_eq!(circuit.supported_parameter(), ParameterType::DKG);
        assert!(circuit.dkg_input_type().is_some());
        assert_eq!(
            circuit.n_recursive_proofs(),
            <TestCircuit as Circuit>::N_PROOFS
        );
        assert_eq!(
            circuit.n_public_inputs(),
            <TestCircuit as Circuit>::N_PUBLIC_INPUTS
        );
    }
}
