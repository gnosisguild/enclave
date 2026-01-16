//! # lib - Enclave Zero-Knowledge Cryptographic Library
//! - **`core`**: Core functions and structs for Public Verifiable
//!   Secret Sharing (PVSS) circuits implementations.
//! - **`math`**: Mathematical utilities including polynomial operations, SAFE sponge
//!   API implementation, and helper functions for circuit construction.
//! - **`configs`**: Cryptographic parameter configurations for different security
//!   levels (insecure, production) and different circuit variants (BFV, TRBFV).

pub mod math;
pub mod core;
pub mod configs;
