// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::builder::build_pair_for_preset;
use crate::builder::{build_bfv_params_from_set, build_bfv_params_from_set_arc};
use crate::constants::{
    insecure_512,
    search_defaults::{B, B_CHI},
    secure_8192,
};
use shared::{SecurityLevel, DEFAULT_INSECURE_LAMBDA, DEFAULT_SECURE_LAMBDA};
use std::sync::Arc;
use thiserror::Error as ThisError;

use fhe::bfv::BfvParameters;

/// BFV preset configurations for PVSS (Public Verifiable Secret Sharing)
///
/// In the PVSS protocol, two types of BFV parameters are needed:
///
/// **Threshold BFV Parameters**: Used for the main threshold encryption/decryption operations
/// (Phases 2-3-4). These are the parameters for the threshold public key that users encrypt with,
/// and for threshold decryption where T+1 parties collaborate to decrypt.
///
/// **DKG Parameters**: Used during Distributed Key Generation (Phases 0-1). Each ciphernode
/// generates a standard (non-threshold) BFV key-pair using these parameters. These keys are
/// used exclusively for encrypting secret shares during DKG, since the threshold public key
/// doesn't exist yet. After DKG completes, these keys are no longer needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum BfvPreset {
    /// Insecure threshold BFV parameters (degree 512) - DO NOT USE IN PRODUCTION
    ///
    /// Used for threshold encryption (GRECO) and threshold decryption operations.
    /// These parameters define the threshold public key that data providers use to encrypt inputs.
    InsecureThresholdBfv512,
    /// Insecure DKG parameters (degree 512) - DO NOT USE IN PRODUCTION
    ///
    /// Used during Phase 0-1 (BFV Key Setup and DKG) where each ciphernode generates
    /// a standard BFV key-pair to encrypt secret shares. These are temporary keys used
    /// only during the key generation process.
    InsecureDkg512,
    /// Secure threshold BFV parameters (degree 8192) - PRODUCTION READY
    ///
    /// Used for threshold encryption (GRECO) and threshold decryption operations.
    /// These parameters define the threshold public key that data providers use to encrypt inputs.
    #[default]
    SecureThresholdBfv8192,
    /// Secure DKG parameters (degree 8192) - PRODUCTION READY
    ///
    /// Used during Phase 0-1 (BFV Key Setup and DKG) where each ciphernode generates
    /// a standard BFV key-pair to encrypt secret shares. These are temporary keys used
    /// only during the key generation process.
    SecureDkg8192,
}

/// Metadata describing a BFV preset configuration
///
/// This struct contains high-level information about a preset, including
/// its security properties and basic parameter dimensions.
#[derive(Debug, Clone, Copy)]
pub struct PresetMetadata {
    /// The canonical name of the preset (e.g., "INSECURE_THRESHOLD_BFV_512")
    pub name: &'static str,
    /// Security level classification (Secure if λ ≥ 80, Insecure otherwise)
    pub security_level: SecurityLevel,
    /// LWE dimension (d) - the degree of the polynomial ring, must be a power of 2
    ///
    /// This determines the size of the polynomial ring R_q = Z_q[X]/(X^d + 1).
    /// Common values are 512, 1024, 2048, 4096, 8192, etc.
    pub degree: usize,
    /// Number of parties (n) - the number of ciphernodes in the system supported by
    /// the preset.
    ///
    /// This affects the security analysis and noise bounds.
    pub num_parties: u128,
    /// Statistical security parameter λ (negl(λ) = 2^{-λ})
    ///
    /// Higher values provide stronger security guarantees but may require
    /// larger parameters. Typically 80 for secure presets, 2 for insecure.
    pub lambda: usize,
}

/// Default search parameters for BFV parameter generation
///
/// These values are used when searching for optimal BFV parameters using
/// the crypto_params search algorithm. They define the constraints and
/// requirements for parameter selection.
///
/// See `crypto_params::bfv::BfvSearchConfig` for more details.
#[derive(Debug, Clone, Copy)]
pub struct PresetSearchDefaults {
    /// Number of parties (n) - the number of ciphernodes in the system supported by
    /// the preset.
    ///
    /// This parameter affects the security analysis and noise bounds.
    pub n: u128,
    /// Number of fresh ciphertext additions z
    ///
    /// Note that the BFV plaintext modulus k will be defined as k = z.
    /// This is also equal to k_plain_eff in the search result.
    pub z: u128,
    /// Plaintext modulus k (plaintext space)
    ///
    /// The modulus for the plaintext space. Typically set equal to z.
    pub k: u128,
    /// Statistical Security parameter λ (negl(λ) = 2^{-λ})
    ///
    /// Higher values provide stronger security guarantees but may require
    /// larger parameters. Typically 80 for secure presets, 2 for insecure.
    pub lambda: u32,
    /// Bound B on the error distribution ψ
    ///
    /// Used to generate e1 when encrypting (e.g., 20 for CBD with σ≈3.2).
    /// This bound is used in security analysis equations.
    pub b: u128,
    /// Bound B_χ on the distribution χ
    ///
    /// Used to generate the secret key sk_i of each party i.
    /// This bound is used in security analysis equations.
    pub b_chi: u128,
}

#[derive(ThisError, Debug)]
pub enum PresetError {
    #[error("Unknown preset: {0}")]
    UnknownPreset(String),
    #[error("Preset does not define a TRBFV/BFV pair: {0}")]
    MissingPair(&'static str),
}

/// A complete BFV parameter set definition
///
/// This struct contains all the values needed to construct a `BfvParameters`
/// instance. It represents a concrete set of cryptographic parameters for
/// building a BFV (Brakerski-Fan-Vercauteren) homomorphic encryption.
#[derive(Debug, Clone, Copy)]
pub struct BfvParamSet {
    /// LWE dimension (d) - the degree of the polynomial ring, must be a power of 2
    ///
    /// This determines the size of the polynomial ring R_q = Z_q[X]/(X^d + 1).
    /// Common values are 512, 1024, 2048, 4096, 8192, etc.
    pub degree: usize,
    /// Plaintext modulus (k) - the modulus for the plaintext space
    ///
    /// This defines the range of values that can be encrypted as plaintexts.
    /// Plaintexts are elements of the ring Z_k.
    pub plaintext_modulus: u64,
    /// Ciphertext moduli (q_i) - array of NTT-friendly primes for the ciphertext space
    ///
    /// These are the moduli used in the Chinese Remainder Theorem (CRT) representation
    /// of the ciphertext space. The product q = ∏q_i is the ciphertext modulus.
    /// Each prime must be NTT-friendly (typically 40-63 bits) for efficient operations.
    pub moduli: &'static [u64],
    /// Error1 variance (as decimal string) - variance of the encryption error distribution
    ///
    /// This is the variance of the error term e0 in the encryption process.
    /// If None, defaults to "10" (the standard default for BFV parameters).
    /// This value is used in noise analysis and security proofs.
    pub error1_variance: Option<&'static str>,
}

impl BfvParamSet {
    pub fn build(self) -> BfvParameters {
        build_bfv_params_from_set(self)
    }

    pub fn build_arc(self) -> Arc<BfvParameters> {
        build_bfv_params_from_set_arc(self)
    }
}

impl BfvPreset {
    pub const ALL: [BfvPreset; 4] = [
        BfvPreset::InsecureThresholdBfv512,
        BfvPreset::InsecureDkg512,
        BfvPreset::SecureThresholdBfv8192,
        BfvPreset::SecureDkg8192,
    ];

    pub const PAIR_PRESETS: [BfvPreset; 2] = [
        BfvPreset::InsecureThresholdBfv512,
        BfvPreset::SecureThresholdBfv8192,
    ];

    pub fn from_name(name: &str) -> Result<Self, PresetError> {
        let normalized = name.trim().to_ascii_uppercase();
        match normalized.as_str() {
            "INSECURE_THRESHOLD_BFV_512" => Ok(Self::InsecureThresholdBfv512),
            "INSECURE_DKG_512" => Ok(Self::InsecureDkg512),
            "SECURE_THRESHOLD_BFV_8192" => Ok(Self::SecureThresholdBfv8192),
            "SECURE_DKG_8192" => Ok(Self::SecureDkg8192),
            _ => Err(PresetError::UnknownPreset(name.to_string())),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            BfvPreset::InsecureThresholdBfv512 => "INSECURE_THRESHOLD_BFV_512",
            BfvPreset::InsecureDkg512 => "INSECURE_DKG_512",
            BfvPreset::SecureThresholdBfv8192 => "SECURE_THRESHOLD_BFV_8192",
            BfvPreset::SecureDkg8192 => "SECURE_DKG_8192",
        }
    }

    pub fn list() -> Vec<&'static str> {
        Self::ALL.iter().map(BfvPreset::name).collect()
    }

    pub fn list_pairs() -> Vec<&'static str> {
        Self::PAIR_PRESETS.iter().map(BfvPreset::name).collect()
    }

    pub fn supports_pair(&self) -> bool {
        Self::PAIR_PRESETS.contains(self)
    }

    pub fn metadata(&self) -> PresetMetadata {
        match self {
            BfvPreset::InsecureThresholdBfv512 | BfvPreset::InsecureDkg512 => PresetMetadata {
                name: self.name(),
                security_level: SecurityLevel::from_lambda(DEFAULT_INSECURE_LAMBDA),
                degree: insecure_512::DEGREE,
                num_parties: insecure_512::NUM_PARTIES,
                lambda: DEFAULT_INSECURE_LAMBDA,
            },
            BfvPreset::SecureThresholdBfv8192 | BfvPreset::SecureDkg8192 => PresetMetadata {
                name: self.name(),
                security_level: SecurityLevel::from_lambda(DEFAULT_SECURE_LAMBDA),
                degree: secure_8192::DEGREE,
                num_parties: secure_8192::NUM_PARTIES,
                lambda: DEFAULT_SECURE_LAMBDA,
            },
        }
    }

    pub fn search_defaults(&self) -> Option<PresetSearchDefaults> {
        match self {
            BfvPreset::InsecureThresholdBfv512 => Some(PresetSearchDefaults {
                n: insecure_512::threshold::SEARCH_N,
                k: insecure_512::threshold::SEARCH_K,
                z: insecure_512::threshold::SEARCH_Z,
                lambda: DEFAULT_INSECURE_LAMBDA as u32,
                b: B,
                b_chi: B_CHI,
            }),
            BfvPreset::SecureThresholdBfv8192 => Some(PresetSearchDefaults {
                n: secure_8192::threshold::SEARCH_N,
                k: secure_8192::threshold::SEARCH_K,
                z: secure_8192::threshold::SEARCH_Z,
                lambda: DEFAULT_SECURE_LAMBDA as u32,
                b: B,
                b_chi: B_CHI,
            }),
            _ => None,
        }
    }

    pub fn build_pair(&self) -> Result<(Arc<BfvParameters>, Arc<BfvParameters>), PresetError> {
        build_pair_for_preset(*self)
    }
}

impl From<BfvPreset> for BfvParamSet {
    fn from(value: BfvPreset) -> Self {
        match value {
            BfvPreset::InsecureThresholdBfv512 => BfvParamSet {
                degree: insecure_512::DEGREE,
                moduli: insecure_512::threshold::MODULI,
                plaintext_modulus: insecure_512::threshold::PLAINTEXT_MODULUS,
                error1_variance: Some(insecure_512::threshold::ERROR1_VARIANCE),
            },
            BfvPreset::InsecureDkg512 => BfvParamSet {
                degree: insecure_512::DEGREE,
                moduli: insecure_512::dkg::MODULI,
                plaintext_modulus: insecure_512::dkg::PLAINTEXT_MODULUS,
                error1_variance: insecure_512::dkg::ERROR1_VARIANCE,
            },
            BfvPreset::SecureThresholdBfv8192 => BfvParamSet {
                degree: secure_8192::DEGREE,
                plaintext_modulus: secure_8192::threshold::PLAINTEXT_MODULUS,
                moduli: secure_8192::threshold::MODULI,
                error1_variance: Some(secure_8192::threshold::ERROR1_VARIANCE),
            },
            BfvPreset::SecureDkg8192 => BfvParamSet {
                degree: secure_8192::DEGREE,
                plaintext_modulus: secure_8192::dkg::PLAINTEXT_MODULUS,
                moduli: secure_8192::dkg::MODULI,
                error1_variance: secure_8192::dkg::ERROR1_VARIANCE,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::{insecure_512, secure_8192};

    #[test]
    fn from_name_accepts_all_presets() {
        for preset in BfvPreset::ALL {
            let parsed = BfvPreset::from_name(preset.name()).expect("preset should parse");
            assert_eq!(parsed, preset);
        }
    }

    #[test]
    fn build_pair_matches_expected_params() {
        let (trbfv, bfv) = BfvPreset::InsecureThresholdBfv512.build_pair().unwrap();
        assert_eq!(trbfv.degree(), insecure_512::DEGREE);
        assert_eq!(
            trbfv.plaintext(),
            insecure_512::threshold::PLAINTEXT_MODULUS
        );
        assert_eq!(trbfv.moduli(), insecure_512::threshold::MODULI);
        assert_eq!(bfv.degree(), insecure_512::DEGREE);
        assert_eq!(bfv.plaintext(), insecure_512::dkg::PLAINTEXT_MODULUS);
        assert_eq!(bfv.moduli(), insecure_512::dkg::MODULI);

        let (trbfv, bfv) = BfvPreset::SecureThresholdBfv8192.build_pair().unwrap();
        assert_eq!(trbfv.degree(), secure_8192::DEGREE);
        assert_eq!(trbfv.plaintext(), secure_8192::threshold::PLAINTEXT_MODULUS);
        assert_eq!(trbfv.moduli(), secure_8192::threshold::MODULI);
        assert_eq!(bfv.degree(), secure_8192::DEGREE);
        assert_eq!(bfv.plaintext(), secure_8192::dkg::BFV_PLAINTEXT_MODULUS);
        assert_eq!(bfv.moduli(), secure_8192::dkg::BFV_MODULI);
    }

    #[test]
    fn test_param_set_build() {
        let preset = BfvPreset::InsecureDkg512;
        let param_set: BfvParamSet = preset.into();

        assert_eq!(param_set.degree, insecure_512::DEGREE);
        assert_eq!(
            param_set.plaintext_modulus,
            insecure_512::dkg::PLAINTEXT_MODULUS
        );
        assert_eq!(param_set.moduli, insecure_512::dkg::MODULI);

        let params = param_set.build();
        assert_eq!(params.degree(), param_set.degree);
        assert_eq!(params.plaintext(), param_set.plaintext_modulus);
        assert_eq!(params.moduli(), param_set.moduli);
    }

    #[test]
    fn test_param_set_build_arc() {
        let preset = BfvPreset::SecureDkg8192;
        let param_set: BfvParamSet = preset.into();

        let params = param_set.build_arc();
        assert_eq!(params.degree(), param_set.degree);
        assert_eq!(params.plaintext(), param_set.plaintext_modulus);
        assert_eq!(params.moduli(), param_set.moduli);
    }

    #[test]
    fn test_metadata_values() {
        let insecure = BfvPreset::InsecureThresholdBfv512;
        let metadata = insecure.metadata();
        assert_eq!(metadata.degree, insecure_512::DEGREE);
        assert_eq!(metadata.num_parties, insecure_512::NUM_PARTIES);
        assert_eq!(metadata.lambda, shared::DEFAULT_INSECURE_LAMBDA);

        let secure = BfvPreset::SecureThresholdBfv8192;
        let metadata = secure.metadata();
        assert_eq!(metadata.degree, secure_8192::DEGREE);
        assert_eq!(metadata.num_parties, secure_8192::NUM_PARTIES);
        assert_eq!(metadata.lambda, shared::DEFAULT_SECURE_LAMBDA);
    }

    #[test]
    fn test_search_defaults() {
        let preset = BfvPreset::InsecureThresholdBfv512;
        let defaults = preset.search_defaults().unwrap();
        assert_eq!(defaults.n, insecure_512::threshold::SEARCH_N);
        assert_eq!(defaults.k, insecure_512::threshold::SEARCH_K);
        assert_eq!(defaults.z, insecure_512::threshold::SEARCH_Z);
        assert_eq!(defaults.lambda, shared::DEFAULT_INSECURE_LAMBDA as u32);

        let preset = BfvPreset::SecureThresholdBfv8192;
        let defaults = preset.search_defaults().unwrap();
        assert_eq!(defaults.n, secure_8192::threshold::SEARCH_N);
        assert_eq!(defaults.k, secure_8192::threshold::SEARCH_K);
        assert_eq!(defaults.z, secure_8192::threshold::SEARCH_Z);
        assert_eq!(defaults.lambda, shared::DEFAULT_SECURE_LAMBDA as u32);

        // DKG presets don't have search defaults
        assert!(BfvPreset::InsecureDkg512.search_defaults().is_none());
        assert!(BfvPreset::SecureDkg8192.search_defaults().is_none());
    }
}
