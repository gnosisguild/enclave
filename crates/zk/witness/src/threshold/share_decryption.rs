// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Circuit input data, witness preparation, and canonical Noir encoding.

use crate::commitments::compute_aggregated_shares_commitment;
use crate::commitments::{compute_ciphertext_commitment, compute_partial_decryption_c0_commitment};
use crate::encoding::crt_polynomial_to_toml_json;
use crate::negacyclic::{exact_reduction_quotient, product_for_fold};
use crate::WitnessError;
use e3_committee::CiphernodesCommittee;
use e3_polynomial::CrtPolynomial;
use e3_polynomial::Polynomial;
use e3_polynomial_fhe::FromFhePolynomial as _;
use e3_zk_config::threshold::share_decryption::Configs;
use fhe::bfv::Ciphertext;
use fhe::bfv::PublicKey;
use num_bigint::BigInt;
use num_traits::Zero;
use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use serde::Deserialize;
use serde::Serialize;

/// Threshold share decryption circuit (PVSS #6).
///
/// Verifies correct computation of a party's decryption share with respect to
/// an anchored aggregate secret, fresh bounded noise, and committed pairwise keys.

/// Input to the share decryption circuit: ciphertext, public key, and the party's
/// aggregated secret share (s), fresh noise (e), and computed decryption share (d_share).
pub struct ShareDecryptionCircuitData {
    pub ciphertext: Ciphertext,
    pub public_key: PublicKey,
    pub s: CrtPolynomial,
    pub e: CrtPolynomial,
    /// Signed fresh-noise coefficients, low degree first. Each CRT residue is this integer mod q.
    pub e_signed: Vec<BigInt>,
    pub d_share: CrtPolynomial,
    /// High and low 128-bit limbs of the E3 decryption domain.
    pub domain_hi: u128,
    pub domain_lo: u128,
    /// Committee that sizes the PRF key arrays.
    pub committee: CiphernodesCommittee,
    /// Zero-based party index of the decryptor.
    pub party_idx: u32,
    /// Exactly T+1 strictly increasing one-based decryptor IDs.
    pub decryptors: Vec<u32>,
    /// Outgoing keys indexed by the zero-based recipient; one non-empty entry per slot.
    pub outgoing_prf_keys: Vec<Vec<u8>>,
    /// Incoming keys indexed by the zero-based sender; one non-empty entry per slot.
    pub incoming_prf_keys: Vec<Vec<u8>>,
}

/// Low-degree native \([0, q)\) CRT limbs for `d_commitment`, matching C7's `from_fhe` truncation.
/// In each limb, `d` is reversed+centered (witness layout); native coeff `j` is `uncenter(d[N-1-j])`.
fn d_native_trunc_from_centered_d(
    d: &CrtPolynomial,
    moduli: &[u64],
    degree: usize,
    max_k: usize,
) -> CrtPolynomial {
    let mut limbs = Vec::with_capacity(d.limbs.len());
    for (limb_idx, limb) in d.limbs.iter().enumerate() {
        let q = BigInt::from(moduli[limb_idx]);
        let coeffs = limb.coefficients();
        let mut asc = Vec::with_capacity(max_k);
        for j in 0..max_k {
            let w = &coeffs[degree - 1 - j];
            let u = if w < &BigInt::zero() {
                w + &q
            } else {
                w.clone()
            };
            asc.push(u);
        }
        limbs.push(Polynomial::new(asc));
    }
    CrtPolynomial::new(limbs)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Inputs {
    pub ct0: CrtPolynomial,
    pub ct1: CrtPolynomial,
    pub sk: CrtPolynomial,
    pub e_fresh: CrtPolynomial,
    pub e_signed: Polynomial,
    pub e_quot: CrtPolynomial,
    pub r: CrtPolynomial,
    pub d: CrtPolynomial,
    /// Native truncated `d` per limb (C7-compatible); hashed for public `d_commitment`.
    pub d_native_trunc: CrtPolynomial,
    pub expected_sk_commitment: BigInt,
    pub c0_commitment: BigInt,
    pub ct_commitment: BigInt,
    pub domain_hi: BigInt,
    pub domain_lo: BigInt,
    pub party_idx: u32,
    pub decryptors: Vec<u32>,
    pub outgoing_prf_keys: Vec<Vec<u8>>,
    pub incoming_prf_keys: Vec<Vec<u8>>,
}

impl Inputs {
    pub fn compute(
        config: &Configs,
        data: &ShareDecryptionCircuitData,
    ) -> Result<Self, WitnessError> {
        let threshold_params = &config.params;
        if data.ciphertext.len() != 2 {
            return Err(WitnessError::UnexpectedCiphertextComponents(
                data.ciphertext.len(),
            ));
        }
        for (index, component) in data.ciphertext.iter().enumerate() {
            let coefficients = component.coefficients();
            crate::validation::context(
                &format!("ciphertext[{index}]"),
                component.ctx(),
                coefficients.nrows(),
                coefficients.ncols(),
                threshold_params,
            )?;
        }
        for (name, value) in [("s", &data.s), ("e", &data.e), ("d_share", &data.d_share)] {
            crate::validation::crt(name, value, threshold_params)?;
        }
        if config.max_msg_non_zero_coeffs > threshold_params.degree() {
            return Err(WitnessError::Other(
                "max_msg_non_zero_coeffs exceeds the polynomial degree".into(),
            ));
        }

        let moduli = threshold_params.moduli();
        let n = threshold_params.degree() as u64;

        let committee =
            e3_committee::CiphernodesCommitteeSize::from_n_h(data.committee.n, data.committee.h)
                .map_err(|e| WitnessError::Other(e.to_string()))?;
        if committee.values().threshold != data.committee.threshold {
            return Err(WitnessError::Other(
                "threshold does not match the configured committee".into(),
            ));
        }
        let reconstruction = data.committee.threshold + 1;
        if data.decryptors.len() != reconstruction {
            return Err(WitnessError::Other(format!(
                "decryptor count {} must equal threshold + 1 ({reconstruction})",
                data.decryptors.len()
            )));
        }
        if data.e_signed.len() != n as usize {
            return Err(WitnessError::Other(format!(
                "signed fresh noise has {} coefficients and the degree is {n}",
                data.e_signed.len()
            )));
        }
        if data.party_idx as usize >= data.committee.n
            || !data.decryptors.contains(&(data.party_idx + 1))
            || data
                .decryptors
                .iter()
                .any(|id| *id == 0 || *id as usize > data.committee.n)
            || data.decryptors.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(WitnessError::Other(
                "invalid canonical decryptor set or owner".into(),
            ));
        }
        let preset = e3_fhe_params::BfvPreset::from_threshold_parameters(
            threshold_params.degree(),
            threshold_params.plaintext(),
            threshold_params.moduli(),
        )
        .ok_or_else(|| WitnessError::Other("unsupported threshold parameters".into()))?;
        let noise = e3_fhe_params::fresh_smudging_inputs(preset, data.committee.n)
            .map_err(WitnessError::Other)?;
        let bound = e3_fhe_params::calculate_smudging_bound(
            threshold_params.clone(),
            noise.num_parties,
            noise.num_ciphertexts,
            noise.mult_depth,
            noise.lambda,
        )?;
        if data
            .e_signed
            .iter()
            .any(|coefficient| coefficient.magnitude() > &bound)
        {
            return Err(WitnessError::Other(
                "fresh noise exceeds the configured integer bound".into(),
            ));
        }
        for (name, keys, count) in [
            (
                "outgoing_prf_keys",
                &data.outgoing_prf_keys,
                data.committee.n,
            ),
            (
                "incoming_prf_keys",
                &data.incoming_prf_keys,
                data.committee.n,
            ),
        ] {
            if !keys.is_empty() && keys.len() != count {
                return Err(WitnessError::Other(format!(
                    "{name} has {} entries and the committee needs {count}",
                    keys.len()
                )));
            }
        }

        // Extract and convert ciphertext polynomials
        let ct0 = CrtPolynomial::from_fhe_polynomial(&data.ciphertext[0]);
        let ct1 = CrtPolynomial::from_fhe_polynomial(&data.ciphertext[1]);
        let mask = crate::prf::mask(
            data.party_idx,
            &data.decryptors,
            &data.outgoing_prf_keys,
            &data.incoming_prf_keys,
            &data.ciphertext,
            data.committee.n,
        )?;
        let mask = CrtPolynomial::from_fhe_polynomial(&mask);
        let lambdas = moduli
            .iter()
            .map(|&q| {
                e3_fhe_params::utils::lagrange_at_zero(&data.decryptors, data.party_idx + 1, q)
                    .map(|lambda| {
                        if lambda > (q - 1) / 2 {
                            BigInt::from(lambda) - q
                        } else {
                            BigInt::from(lambda)
                        }
                    })
                    .map_err(WitnessError::Other)
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Perform the main computation logic
        #[allow(clippy::type_complexity)]
        let results: Vec<(
            Polynomial,
            Polynomial,
            Polynomial,
            Polynomial,
            Polynomial,
            Polynomial,
        )> = (
            moduli,
            ct0.limbs,
            ct1.limbs,
            &data.s.limbs,
            &data.e.limbs,
            &data.d_share.limbs,
            &mask.limbs,
            &lambdas,
        )
            .into_par_iter()
            .map(|(qi, mut ct0, mut ct1, s, e, d_share, mask, lambda)| {
                let qi = BigInt::from(*qi);
                let mut s = s.clone();
                let mut e = e.clone();
                let mut d_share = d_share.clone();
                ct0.reverse();
                ct0.center(&qi);

                ct1.reverse();
                ct1.center(&qi);

                s.reverse();
                s.center(&qi);

                e.reverse();
                e.center(&qi);

                d_share.reverse();
                d_share.center(&qi);

                let mut mask = mask.clone();
                mask.reverse();
                // The partial share excludes c0 and applies the Lagrange weight locally.
                let d_share_hat = {
                    // ct1 * s (degree 2*(n-1))
                    let ct1_s_times = product_for_fold(&ct1, &s, n as usize);
                    assert_eq!((ct1_s_times.coefficients().len() as u64) - 1, 2 * (n - 1));

                    // ct0 + ct1 * s + e
                    ct1_s_times.scalar_mul(lambda).add(&e).add(&mask)
                };
                assert_eq!((d_share_hat.coefficients().len() as u64) - 1, 2 * (n - 1));

                // The circuit checks the identity reduced modulo X^N + 1, so the cyclotomic
                // quotient's term is identically zero and `r2` is gone. `r` is then pinned by the
                // identity itself:
                //   d = (ct0 + ct1 * s + e mod X^N + 1) + q * r
                // so folding the `d_share_hat` already computed above and dividing by q gives it.
                let (reduced_hat, r) =
                    exact_reduction_quotient(&d_share, &d_share_hat, n as usize, &qi)?;

                // Check the derived witness against the integer identity.
                assert!(
                    d_share.sub(&reduced_hat.add(&r.scalar_mul(&qi))).is_zero(),
                    "synchronized partial-share identity must hold over the integers"
                );

                Ok((ct0, ct1, s, e, d_share, r))
            })
            .collect::<Result<Vec<_>, WitnessError>>()?;

        let mut ct0 = CrtPolynomial::new(vec![]);
        let mut ct1 = CrtPolynomial::new(vec![]);
        let mut sk = CrtPolynomial::new(vec![]);
        let mut e_sm = CrtPolynomial::new(vec![]);
        let mut r = CrtPolynomial::new(vec![]);
        let mut d = CrtPolynomial::new(vec![]);

        for (ct0i, ct1i, si, ei, d_sharei, ri) in results {
            ct0.add_limb(ct0i);
            ct1.add_limb(ct1i);
            sk.add_limb(si);
            e_sm.add_limb(ei);
            r.add_limb(ri);
            d.add_limb(d_sharei);
        }

        // Signed fresh noise must reduce to the centered witness residue in every limb.
        let mut signed_coeffs = data.e_signed.clone();
        signed_coeffs.reverse();
        let e_signed = Polynomial::new(signed_coeffs);
        let mut e_quot = CrtPolynomial::new(vec![]);
        for (limb_idx, limb) in e_sm.limbs.iter().enumerate() {
            let q = BigInt::from(moduli[limb_idx]);
            let quotients = e_signed
                .coefficients()
                .iter()
                .zip(limb.coefficients())
                .map(|(coeff, residue)| {
                    let diff = coeff - residue;
                    if (&diff % &q).is_zero() {
                        Ok(diff / &q)
                    } else {
                        Err(WitnessError::Other(
                            "fresh noise residue does not match the signed coefficient".into(),
                        ))
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;
            e_quot.add_limb(Polynomial::new(quotients));
        }

        // Compute commitments to s and e (matches circuit's commitment functions)
        let modulus_bit = config.bits.sk_bit;
        let expected_sk_commitment = compute_aggregated_shares_commitment(&sk, modulus_bit);
        let bits = &config.bits;
        let ct_commitment = compute_ciphertext_commitment(&ct0, &ct1, bits.ct_bit);
        let native_ct0 = CrtPolynomial::from_fhe_polynomial(&data.ciphertext[0]);
        let c0_commitment = compute_partial_decryption_c0_commitment(
            &native_ct0,
            bits.d_native_bit,
            config.max_msg_non_zero_coeffs,
        );

        let d_native_trunc =
            d_native_trunc_from_centered_d(&d, moduli, n as usize, config.max_msg_non_zero_coeffs);

        Ok(Inputs {
            ct0,
            ct1,
            sk,
            e_fresh: e_sm,
            e_signed,
            e_quot,
            r,
            d,
            d_native_trunc,
            expected_sk_commitment,
            c0_commitment,
            ct_commitment,
            domain_hi: BigInt::from(data.domain_hi),
            domain_lo: BigInt::from(data.domain_lo),
            party_idx: data.party_idx,
            decryptors: data.decryptors.clone(),
            outgoing_prf_keys: data.outgoing_prf_keys.clone(),
            incoming_prf_keys: data.incoming_prf_keys.clone(),
        })
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "ct0": crt_polynomial_to_toml_json(&self.ct0),
            "ct1": crt_polynomial_to_toml_json(&self.ct1),
            "sk": crt_polynomial_to_toml_json(&self.sk),
            "e_fresh": crt_polynomial_to_toml_json(&self.e_fresh),
            "e_signed": crate::encoding::polynomial_to_toml_json(&self.e_signed),
            "e_quot": crt_polynomial_to_toml_json(&self.e_quot),
            "r": crt_polynomial_to_toml_json(&self.r),
            "d": crt_polynomial_to_toml_json(&self.d),
            "d_native_trunc": crt_polynomial_to_toml_json(&self.d_native_trunc),
            "expected_sk_commitment": self.expected_sk_commitment.to_string(),
            "c0_commitment": self.c0_commitment.to_string(),
            "ct_commitment": self.ct_commitment.to_string(),
            "domain_hi": self.domain_hi.to_string(),
            "domain_lo": self.domain_lo.to_string(),
            "party_idx": self.party_idx,
            "decryptors": self.decryptors,
            "outgoing_keys": self.outgoing_prf_keys.iter().map(|key| crate::prf::key_field(key)).collect::<Vec<_>>(),
            "incoming_keys": self.incoming_prf_keys.iter().map(|key| crate::prf::key_field(key)).collect::<Vec<_>>(),
            "outgoing_key_commitments": self.outgoing_prf_keys.iter().enumerate().map(|(i, key)| crate::prf::key_commitment(i as u32, key).to_string()).collect::<Vec<_>>(),
            "incoming_key_commitments": self.incoming_prf_keys.iter().map(|key| crate::prf::key_commitment(self.party_idx, key).to_string()).collect::<Vec<_>>(),
        })
    }
}

/// Encode signed coefficients as a length-prefixed list of little-endian two's-complement bytes.
pub fn encode_signed_coefficients(coeffs: &[BigInt]) -> Vec<u8> {
    let mut out = Vec::new();
    let count = u32::try_from(coeffs.len()).expect("signed noise coefficient count fits u32");
    out.extend_from_slice(&count.to_le_bytes());
    for coeff in coeffs {
        let bytes = coeff.to_signed_bytes_le();
        let len = u32::try_from(bytes.len()).expect("signed noise coefficient length fits u32");
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&bytes);
    }
    out
}

/// Decode coefficients written by [`encode_signed_coefficients`].
pub fn decode_signed_coefficients(bytes: &[u8]) -> Result<Vec<BigInt>, String> {
    if bytes.len() < 4 {
        return Err("signed noise encoding is too short".into());
    }
    let count = u32::from_le_bytes(bytes[0..4].try_into().expect("4 bytes")) as usize;
    if count > (bytes.len() - 4) / 4 {
        return Err("signed noise coefficient count exceeds the encoded data".into());
    }
    let mut offset = 4;
    let mut coeffs = Vec::with_capacity(count);
    for _ in 0..count {
        if offset + 4 > bytes.len() {
            return Err("signed noise encoding ended early".into());
        }
        let len =
            u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("4 bytes")) as usize;
        offset += 4;
        if offset + len > bytes.len() {
            return Err("signed noise coefficient ended early".into());
        }
        coeffs.push(BigInt::from_signed_bytes_le(&bytes[offset..offset + len]));
        offset += len;
    }
    if offset != bytes.len() {
        return Err("signed noise encoding has trailing bytes".into());
    }
    Ok(coeffs)
}

#[cfg(test)]
mod noise_encoding_tests {
    use super::*;

    #[test]
    fn signed_noise_round_trips_without_losing_sign_or_width() {
        let coefficients = vec![BigInt::from(0), BigInt::from(-3), BigInt::from(1) << 140];
        assert_eq!(
            decode_signed_coefficients(&encode_signed_coefficients(&coefficients)).unwrap(),
            coefficients
        );
    }

    #[test]
    fn malformed_counts_lengths_and_trailing_data_are_rejected() {
        assert!(decode_signed_coefficients(&u32::MAX.to_le_bytes()).is_err());
        let mut truncated = 1u32.to_le_bytes().to_vec();
        truncated.extend_from_slice(&u32::MAX.to_le_bytes());
        assert!(decode_signed_coefficients(&truncated).is_err());
        let mut trailing = encode_signed_coefficients(&[BigInt::from(-1)]);
        trailing.push(0);
        assert!(decode_signed_coefficients(&trailing).is_err());
    }
}
