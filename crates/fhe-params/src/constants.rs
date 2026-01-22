//! Constants for BFV presets
//!
//! This module contains all hardcoded values used in preset definitions.
//! Centralizing these values makes it easier to maintain and update presets.

/// Insecure preset constants (degree 512) - DO NOT USE IN PRODUCTION
pub mod insecure_512 {
    pub const DEGREE: usize = 512;
    pub const NUM_PARTIES: u128 = 5;

    /// Threshold BFV (TRBFV) parameters
    pub mod threshold {
        pub const PLAINTEXT_MODULUS: u64 = 10;
        pub const MODULI: &[u64] = &[0xffffee001, 0xffffc4001];
        pub const ERROR1_VARIANCE: &str = "3";
        pub const ERROR1_VARIANCE_BIGUINT: u32 = 3;

        /// Search defaults for insecure threshold BFV
        pub const SEARCH_N: u128 = 5;
        pub const SEARCH_K: u128 = 1000;
        pub const SEARCH_Z: u128 = 1000;
    }

    /// DKG parameters
    pub mod dkg {
        pub const PLAINTEXT_MODULUS: u64 = 0xffffee001;
        pub const MODULI: &[u64] = &[0x7fffffffe0001];
        pub const ERROR1_VARIANCE: Option<&str> = None;
        pub const VARIANCE: u32 = 3;
    }
}

/// Secure preset constants (degree 8192) - PRODUCTION READY
pub mod secure_8192 {
    pub const DEGREE: usize = 8192;
    pub const NUM_PARTIES: u128 = 100;

    /// Threshold BFV (TRBFV) parameters
    pub mod threshold {
        pub const PLAINTEXT_MODULUS: u64 = 100;
        pub const MODULI: &[u64] = &[
            0x0008000000820001,
            0x0010000000060001,
            0x00100000003e0001,
            0x00100000006e0001,
        ];
        pub const ERROR1_VARIANCE: &str =
            "1004336277661868922213726307713258317841382576849282939643494400";

        /// Search defaults for secure threshold BFV
        pub const SEARCH_N: u128 = 100;
        pub const SEARCH_K: u128 = 100;
        pub const SEARCH_Z: u128 = 100;
    }

    /// DKG parameters
    pub mod dkg {
        pub const PLAINTEXT_MODULUS: u64 = 144115188075855872;
        pub const MODULI: &[u64] = &[288230376173076481, 288230376167047169];
        pub const ERROR1_VARIANCE: Option<&str> = None;

        /// BFV plaintext modulus for pair building
        pub const BFV_PLAINTEXT_MODULUS: u64 = 18014398509481984;
        pub const BFV_MODULI: &[u64] = &[0x0100000002a20001, 0x0100000001760001];
    }
}

/// Common search defaults shared across presets
pub mod search_defaults {
    pub const B: u128 = 20;
    pub const B_CHI: u128 = 1;
}

/// Default values for BFV parameters
pub mod defaults {
    /// Default variance for BFV parameters when not explicitly set
    /// This is the standard default variance (and error1_variance) used in BFV
    /// when variance is not specified. Both variance() and error1_variance default to this value.
    pub const VARIANCE: usize = 10;
    pub const ERROR1_VARIANCE: u32 = 10;
}
