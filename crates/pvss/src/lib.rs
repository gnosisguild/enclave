// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! PVSS library for circuit computation and registry utilities.
//!
//! This crate exposes runtime-facing APIs for circuit selection and computation.

pub mod circuits;
pub mod codegen;
pub mod computation;
pub mod registry;

pub use registry::{CircuitRegistry, RegistryError};
