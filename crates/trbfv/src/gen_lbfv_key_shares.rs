// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use anyhow::{ensure, Context, Result};
use derivative::Derivative;
use e3_crypto::{Cipher, SensitiveBytes};
use e3_fhe_params::{lbfv_crs_seed, lbfv_urs_seed, BfvPreset};
use e3_utils::ArcBytes;
use fhe::bfv::{BfvParameters, SecretKey};
use fhe_math::rq::{Poly, PowerBasis};
use rand::{CryptoRng, RngCore};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use zeroize::Zeroizing;

use crate::helpers::try_poly_pb_from_bytes;
use crate::lbfv_operation::{LbfvOperationId, LbfvOperationKind};

/// Generate one l-BFV public-key share and one relinearization-key share.
#[derive(Derivative, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derivative(Debug)]
pub struct GenLbfvKeySharesRequest {
    pub operation_id: LbfvOperationId,
    /// Canonical l-BFV proof-session identifier.
    pub session_id: [u8; 32],
    /// Zero-based party ID in the finalized committee.
    pub party_id: u32,
    /// The encrypted level-0 secret-key polynomial used by the legacy C1 request.
    #[derivative(Debug = "ignore")]
    pub secret_key_bytes: SensitiveBytes,
    /// The encrypted 32-byte seed for deterministic generation retries.
    #[derivative(Debug = "ignore")]
    pub generation_seed: SensitiveBytes,
    pub params_preset: BfvPreset,
    pub ciphertext_level: u32,
    pub key_level: u32,
}

impl GenLbfvKeySharesRequest {
    /// Recompute the operation identity from request semantics.
    pub fn expected_operation_id(&self) -> LbfvOperationId {
        LbfvOperationId::new(
            self.session_id,
            self.party_id,
            LbfvOperationKind::GenKeyShares,
            None,
            None,
            None,
        )
    }

    /// Recompute and validate the operation identity from request semantics.
    pub fn validate_operation_id(&self) -> Result<()> {
        ensure!(
            lbfv_crs_seed(self.params_preset).is_some()
                && lbfv_urs_seed(self.params_preset).is_some(),
            "l-BFV key-share generation requires a preset with l-BFV constants"
        );
        ensure!(
            self.ciphertext_level == 0 && self.key_level == 0,
            "l-BFV key-share generation supports only ciphertext level 0 and key level 0"
        );
        ensure!(
            self.operation_id == self.expected_operation_id(),
            "l-BFV key-share operation ID does not match the request semantics"
        );
        Ok(())
    }
}

/// The encrypted private values required to prove each RLK row.
#[derive(Derivative, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derivative(Debug)]
pub struct EncryptedRlkWitness {
    #[derivative(Debug = "ignore")]
    pub r_bytes: SensitiveBytes,
    #[derivative(Debug = "ignore")]
    pub errors_d0_bytes: Vec<SensitiveBytes>,
    #[derivative(Debug = "ignore")]
    pub errors_d2_bytes: Vec<SensitiveBytes>,
}

/// Public l-BFV shares and the encrypted local RLK proof witness.
#[derive(Derivative, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[derivative(Debug)]
pub struct GenLbfvKeySharesResponse {
    pub operation_id: LbfvOperationId,
    pub public_key_share_bytes: ArcBytes,
    pub rlk_share_bytes: ArcBytes,
    #[derivative(Debug = "ignore")]
    pub witness: EncryptedRlkWitness,
}

/// Reconstruct the ternary BFV secret key from the level-0 RNS polynomial used by C1.
pub fn deserialize_c1_secret_key(bytes: &[u8], params: &Arc<BfvParameters>) -> Result<SecretKey> {
    let poly: Zeroizing<Poly<PowerBasis>> = Zeroizing::new(try_poly_pb_from_bytes(bytes, params)?);
    let coefficients = poly.coefficients();
    ensure!(
        coefficients.nrows() == params.moduli().len() && coefficients.ncols() == params.degree(),
        "C1 secret-key polynomial has an invalid shape"
    );

    let first_modulus = params.moduli()[0];
    let mut secret_coefficients = Zeroizing::new(Vec::with_capacity(params.degree()));
    for coefficient_index in 0..params.degree() {
        let residue = coefficients[(0, coefficient_index)];
        let centered = if residue <= first_modulus / 2 {
            i64::try_from(residue).context("C1 secret-key coefficient does not fit i64")?
        } else {
            -i64::try_from(first_modulus - residue)
                .context("C1 secret-key coefficient does not fit i64")?
        };
        ensure!(
            (-1..=1).contains(&centered),
            "C1 secret-key polynomial contains a non-ternary coefficient"
        );

        for (row_index, modulus) in params.moduli().iter().copied().enumerate() {
            let expected = match centered {
                -1 => modulus - 1,
                0 => 0,
                1 => 1,
                _ => unreachable!("the ternary range was checked"),
            };
            ensure!(
                coefficients[(row_index, coefficient_index)] == expected,
                "C1 secret-key polynomial has inconsistent RNS limbs"
            );
        }
        secret_coefficients.push(centered);
    }

    Ok(SecretKey::new(secret_coefficients.to_vec(), params))
}

/// Generate secure-16384 l-BFV key shares from the secret contribution used by C1.
pub fn gen_lbfv_key_shares<R: RngCore + CryptoRng>(
    _rng: &mut R,
    _cipher: &Cipher,
    request: GenLbfvKeySharesRequest,
) -> Result<GenLbfvKeySharesResponse> {
    request.validate_operation_id()?;
    anyhow::bail!("l-BFV key shares are not available on this fhe.rs branch")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn operation_id(session_id: [u8; 32], party_id: u32) -> LbfvOperationId {
        LbfvOperationId::new(
            session_id,
            party_id,
            LbfvOperationKind::GenKeyShares,
            None,
            None,
            None,
        )
    }

    #[tokio::test]
    async fn rejects_other_presets_and_nonzero_levels_before_decryption() -> Result<()> {
        let cipher = Cipher::from_password("lbfv-key-share-errors").await?;
        let mut rng = rand::rng();
        for request in [
            GenLbfvKeySharesRequest {
                operation_id: operation_id([7; 32], 0),
                session_id: [7; 32],
                party_id: 0,
                secret_key_bytes: SensitiveBytes::from_encrypted(&[]),
                generation_seed: SensitiveBytes::from_encrypted(&[]),
                params_preset: BfvPreset::SecureThreshold8192,
                ciphertext_level: 0,
                key_level: 0,
            },
            GenLbfvKeySharesRequest {
                operation_id: operation_id([7; 32], 0),
                session_id: [7; 32],
                party_id: 0,
                secret_key_bytes: SensitiveBytes::from_encrypted(&[]),
                generation_seed: SensitiveBytes::from_encrypted(&[]),
                params_preset: BfvPreset::SecureThreshold16384,
                ciphertext_level: 1,
                key_level: 0,
            },
            GenLbfvKeySharesRequest {
                operation_id: operation_id([7; 32], 0),
                session_id: [7; 32],
                party_id: 0,
                secret_key_bytes: SensitiveBytes::from_encrypted(&[]),
                generation_seed: SensitiveBytes::from_encrypted(&[]),
                params_preset: BfvPreset::SecureThreshold16384,
                ciphertext_level: 0,
                key_level: 1,
            },
        ] {
            assert!(gen_lbfv_key_shares(&mut rng, &cipher, request).is_err());
        }
        Ok(())
    }

    #[test]
    fn operation_id_binds_session_and_party() {
        let session_id = [7; 32];
        let mut request = GenLbfvKeySharesRequest {
            operation_id: operation_id(session_id, 1),
            session_id,
            party_id: 1,
            secret_key_bytes: SensitiveBytes::from_encrypted(&[]),
            generation_seed: SensitiveBytes::from_encrypted(&[]),
            params_preset: BfvPreset::SecureThreshold16384,
            ciphertext_level: 0,
            key_level: 0,
        };

        request.validate_operation_id().unwrap();
        request.party_id = 2;
        assert!(request.validate_operation_id().is_err());
    }

    #[test]
    fn debug_omits_the_encrypted_witness_fields() {
        let response = GenLbfvKeySharesResponse {
            operation_id: LbfvOperationId([7; 32]),
            public_key_share_bytes: ArcBytes::from_bytes(&[]),
            rlk_share_bytes: ArcBytes::from_bytes(&[]),
            witness: EncryptedRlkWitness {
                r_bytes: SensitiveBytes::from_encrypted(b"r-secret"),
                errors_d0_bytes: vec![SensitiveBytes::from_encrypted(b"d0-secret")],
                errors_d2_bytes: vec![SensitiveBytes::from_encrypted(b"d2-secret")],
            },
        };
        let debug = format!("{response:?}");
        assert!(!debug.contains("witness"));
        assert!(!debug.contains("r_bytes"));
        assert!(!debug.contains("errors_d0_bytes"));
        assert!(!debug.contains("errors_d2_bytes"));
        assert!(!debug.contains("secret"));
    }
}
