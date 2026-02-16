// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Constants for BFV presets
//!
//! This module contains all hardcoded values used in preset definitions.
//! Centralizing these values makes it easier to maintain and update presets.

/// Insecure preset constants (degree 512) - DO NOT USE IN PRODUCTION
pub mod insecure_512 {
    pub const DEGREE: usize = 512;
    pub const NUM_PARTIES: u128 = 5; // fake - not used in the search default

    /// Threshold BFV parameters
    pub mod threshold {
        pub const PLAINTEXT_MODULUS: u64 = 100;
        pub const MODULI: &[u64] = &[0xffffee001, 0xffffc4001];
        pub const ERROR1_VARIANCE: &str = "3";
        pub const ERROR1_VARIANCE_BIGUINT: u32 = 3;
    }

    /// DKG parameters
    pub mod dkg {
        pub const PLAINTEXT_MODULUS: u64 = 0xffffee001;
        pub const MODULI: &[u64] = &[0x7fffffffe0001];
        pub const ERROR1_VARIANCE: &str = "10";
        pub const VARIANCE: u32 = 3;
    }
}

/// Secure preset constants (degree 8192) - PRODUCTION READY
pub mod secure_8192 {
    pub const DEGREE: usize = 8192;
    pub const NUM_PARTIES: u128 = 100; // real - used in the search default

    /// Threshold BFV parameters
    pub mod threshold {
        pub const PLAINTEXT_MODULUS: u64 = 100;
        pub const MODULI: &[u64] = &[
            0x0008000000820001,
            0x0010000000060001,
            0x00100000003e0001,
            0x00100000006e0001,
        ];
        pub const ERROR1_VARIANCE: &str =
            "523091811282223396986315785267318739368948664428268466733056000";
    }

    /// DKG parameters
    pub mod dkg {
        pub const PLAINTEXT_MODULUS: u64 = 18014398509481984;
        pub const MODULI: &[u64] = &[0x0100000002a20001, 0x0100000001760001];
        pub const ERROR1_VARIANCE: &str = "10";
    }
}

/// Common search defaults shared across presets
/// These are for the SecureThreshold8192 preset.
/// The InsecureThreshold512 preset has been generated manually.
pub mod search_defaults {
    pub const B: u128 = 20;
    pub const B_CHI: u128 = 1;
    pub const SEARCH_N: u128 = 100;
    pub const SEARCH_K: u128 = 100;
    pub const SEARCH_Z: u128 = 100;
}

/// Default values for BFV parameters
pub mod defaults {
    /// Default variance for BFV parameters when not explicitly set
    /// This is the standard default variance (and error1_variance) used in BFV
    /// when variance is not specified. Both variance() and error1_variance default to this value.
    pub const VARIANCE: usize = 10;

    /// Default insecure security parameter (λ).
    pub const DEFAULT_INSECURE_LAMBDA: usize = 2;
    /// Default secure security parameter (λ).
    pub const DEFAULT_SECURE_LAMBDA: usize = 80;
}
