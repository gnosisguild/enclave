// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! BFV parameter arithmetic for ZK circuit computations.

use crate::CircuitsErrors;
use fhe::bfv::{Encoding, Plaintext};
use fhe_math::zq::Modulus;
use fhe_traits::FheDecoder;
use num_bigint::{BigInt, BigUint};
use num_integer::Integer;
use num_traits::ToPrimitive;

/// Encoded plaintext coefficients in poly encoding (u64 limb values).
pub fn plaintext_poly_u64(pt: &Plaintext) -> Result<Vec<u64>, CircuitsErrors> {
    Vec::<u64>::try_decode(pt, Encoding::poly()).map_err(CircuitsErrors::Fhe)
}

/// Product Q = q_0 * q_1 * ... * q_{L-1} of CRT moduli.
pub fn compute_q_product(moduli: &[u64]) -> BigUint {
    let mut q = BigUint::from(1u64);
    for &m in moduli {
        q *= BigUint::from(m);
    }
    q
}

/// Delta = floor(Q / t) for plaintext modulus t.
pub fn compute_delta(q: &BigUint, t: u64) -> BigUint {
    q / BigUint::from(t)
}

/// Delta_half = floor(delta / 2).
pub fn compute_delta_half(delta: &BigUint) -> BigUint {
    delta / BigUint::from(2u64)
}

/// Q^{-1} mod t (for BFV decoding). Fails if gcd(Q, t) != 1.
pub fn compute_q_inverse_mod_t(q: &BigUint, t: u64) -> Result<u64, CircuitsErrors> {
    let q_bigint = BigInt::from(q.clone());
    let t_bigint = BigInt::from(t);
    let gcd_result = q_bigint.extended_gcd(&t_bigint);
    if gcd_result.gcd != BigInt::from(1) {
        return Err(CircuitsErrors::Other(format!(
            "Q and t are not coprime, gcd = {}",
            gcd_result.gcd
        )));
    }
    let inv = gcd_result.x % &t_bigint;
    let inv_positive = if inv < BigInt::from(0) {
        inv + &t_bigint
    } else {
        inv
    };
    inv_positive.to_u64().ok_or_else(|| {
        CircuitsErrors::Other(format!(
            "q_inverse_mod_t too large to fit in u64: {}",
            inv_positive
        ))
    })
}

/// Q mod t.
pub fn compute_q_mod_t(q: &BigUint, t: u64) -> BigUint {
    q % BigUint::from(t)
}

/// Q mod t in centered form [-t/2, t/2], given CRT moduli and plaintext modulus t.
/// Use with threshold or DKG params via `params.moduli()` and `params.plaintext()`.
pub fn compute_q_mod_t_centered(moduli: &[u64], t: u64) -> BigInt {
    let q = compute_q_product(moduli);
    let q_mod_t_uint = compute_q_mod_t(&q, t);
    let t_bn = BigInt::from(t);
    // Preserve the raw helper's full u64 domain; zq::Modulus is limited to 62 bits.
    let half = &t_bn / 2u32;
    (BigInt::from(q_mod_t_uint) + &half).mod_floor(&t_bn) - half
}

/// t^{-1} mod Q (for CRT / scaling). Fails if gcd(Q, t) != 1.
pub fn compute_t_inv_mod_q(q: &BigUint, t: u64) -> Result<BigUint, CircuitsErrors> {
    let q_bigint = BigInt::from(q.clone());
    let t_bigint = BigInt::from(t);
    let gcd_result = q_bigint.extended_gcd(&t_bigint);
    if gcd_result.gcd != BigInt::from(1) {
        return Err(CircuitsErrors::Other(format!(
            "Q and t are not coprime (t_inv_mod_q), gcd = {}",
            gcd_result.gcd
        )));
    }
    let y = gcd_result.y;
    let t_inv_bigint = if y < BigInt::from(0) {
        y + &q_bigint
    } else {
        y
    };
    t_inv_bigint
        .to_biguint()
        .ok_or_else(|| CircuitsErrors::Other("Failed to convert t_inv_mod_q to BigUint".into()))
}

// ---------- CRT (k0is, FHE poly to CRT) ----------

/// Computes k0_i = (-t)^{-1} mod q_i for each modulus (used in Configs and bounds).
pub fn compute_k0is(moduli: &[u64], plaintext_modulus: u64) -> Result<Vec<u64>, CircuitsErrors> {
    let mut k0is = Vec::with_capacity(moduli.len());
    for &qi in moduli {
        let m = Modulus::new(qi).map_err(|e| {
            CircuitsErrors::Sample(format!("Failed to create modulus for k0is: {:?}", e))
        })?;
        if plaintext_modulus >= qi {
            return Err(CircuitsErrors::Other(format!(
                "plaintext modulus {plaintext_modulus} must be smaller than CRT modulus {qi}"
            )));
        }
        let neg_t = m.neg(plaintext_modulus);
        let k0qi = m.inv(neg_t).ok_or_else(|| {
            CircuitsErrors::Fhe(fhe::Error::MathError(fhe_math::Error::NonInvertible {
                value: neg_t,
                modulus: qi,
            }))
        })?;
        k0is.push(k0qi);
    }
    Ok(k0is)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_q_product() {
        let moduli = [3u64, 5, 7];
        let q = compute_q_product(&moduli);
        assert_eq!(q, BigUint::from(105u64));
    }

    #[test]
    fn centered_q_residues_preserve_small_and_full_width_moduli() {
        for (moduli, t, expected) in [
            (vec![3], 5, BigInt::from(-2)),
            (vec![2], 5, BigInt::from(2)),
            (vec![3], 6, BigInt::from(-3)),
            (vec![1], 1, BigInt::from(0)),
            (vec![1 << 61], 1 << 62, -BigInt::from(1u64 << 61)),
            (vec![u64::MAX / 2], u64::MAX, BigInt::from(u64::MAX / 2)),
            (
                vec![(u64::MAX - 1) / 2],
                u64::MAX - 1,
                -BigInt::from((u64::MAX - 1) / 2),
            ),
            (vec![u64::MAX - 1], u64::MAX, BigInt::from(-1)),
        ] {
            assert_eq!(compute_q_mod_t_centered(&moduli, t), expected);
        }
        for t in [100, 0xffffee001, 17000000, 576460752317792257] {
            let modulus = Modulus::new(t).unwrap();
            for residue in [0, (t - 1) / 2, t / 2 + 1, t - 1] {
                assert_eq!(
                    compute_q_mod_t_centered(&[residue], t),
                    BigInt::from(modulus.center(residue)),
                );
            }
        }
    }

    #[test]
    fn compute_k0is_requires_plaintext_modulus_below_each_crt_modulus() {
        assert_eq!(compute_k0is(&[17], 5).unwrap(), vec![10]);
        assert!(compute_k0is(&[17], 17).is_err());
        assert!(compute_k0is(&[17], 18).is_err());
    }
}
