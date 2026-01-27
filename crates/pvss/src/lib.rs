//! PVSS library for circuit computation and registry utilities.
//!
//! This crate exposes runtime-facing APIs for circuit selection and computation.

pub mod circuits;
pub mod codegen;
pub mod computation;
pub mod registry;

pub use registry::{CircuitRegistry, RegistryError};
