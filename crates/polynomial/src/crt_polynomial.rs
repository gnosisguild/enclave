// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Integer coefficient limbs, with explicit per-limb modular operations.

use crate::Polynomial;
use num_bigint::BigInt;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CrtPolynomialError {
    #[error("moduli length ({moduli_len}) must match limbs length ({limbs_len})")]
    ModuliLengthMismatch { limbs_len: usize, moduli_len: usize },
}

/// One coefficient polynomial per CRT limb. The caller owns the corresponding moduli
/// and coefficient order; this container performs no implicit reduction or reversal.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct CrtPolynomial {
    pub limbs: Vec<Polynomial>,
}

impl CrtPolynomial {
    pub fn new(limbs: Vec<Polynomial>) -> Self {
        Self { limbs }
    }

    /// Split integer coefficients into residues, retaining their input order.
    pub fn from_mod_q_polynomial(poly: &[BigInt], moduli: &[u64]) -> Self {
        Self::new(
            moduli
                .iter()
                .map(|&qi| {
                    let qi = BigInt::from(qi);
                    let mut limb = Polynomial::new(poly.to_vec());
                    limb.reduce(&qi);
                    limb
                })
                .collect(),
        )
    }

    pub fn reverse(&mut self) {
        for limb in &mut self.limbs {
            limb.reverse();
        }
    }

    /// Center each limb that is already reduced modulo its corresponding modulus.
    pub fn center(&mut self, moduli: &[u64]) -> Result<(), CrtPolynomialError> {
        self.check_moduli(moduli)?;
        for (limb, &qi) in self.limbs.iter_mut().zip(moduli) {
            limb.center(&BigInt::from(qi));
        }
        Ok(())
    }

    pub fn reduce(&mut self, moduli: &[u64]) -> Result<(), CrtPolynomialError> {
        self.check_moduli(moduli)?;
        for (limb, &qi) in self.limbs.iter_mut().zip(moduli) {
            limb.reduce(&BigInt::from(qi));
        }
        Ok(())
    }

    fn check_moduli(&self, moduli: &[u64]) -> Result<(), CrtPolynomialError> {
        if self.limbs.len() != moduli.len() {
            return Err(CrtPolynomialError::ModuliLengthMismatch {
                limbs_len: self.limbs.len(),
                moduli_len: moduli.len(),
            });
        }
        Ok(())
    }

    pub fn add_limb(&mut self, limb: Polynomial) {
        self.limbs.push(limb);
    }

    pub fn limb(&self, i: usize) -> &Polynomial {
        &self.limbs[i]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limb_moduli_preserve_order_padding_and_fail_without_partial_changes() {
        let input = [-8, 3, 0].map(BigInt::from);
        let mut crt = CrtPolynomial::from_mod_q_polynomial(&input, &[5, 6]);
        assert_eq!(crt.limb(0).coefficients(), &[2, 3, 0].map(BigInt::from));
        assert_eq!(crt.limb(1).coefficients(), &[4, 3, 0].map(BigInt::from));
        let before = crt.clone();
        assert!(matches!(
            crt.center(&[5]),
            Err(CrtPolynomialError::ModuliLengthMismatch { .. })
        ));
        assert_eq!(crt, before);
        crt.center(&[5, 6]).unwrap();
        crt.reverse();
        assert_eq!(crt.limb(0).coefficients(), &[0, -2, 2].map(BigInt::from));
        assert_eq!(crt.limb(1).coefficients(), &[0, -3, -2].map(BigInt::from));
        crt.reduce(&[5, 6]).unwrap();
        assert_eq!(crt.limb(0).coefficients(), &[0, 3, 2].map(BigInt::from));
        assert_eq!(crt.limb(1).coefficients(), &[0, 3, 4].map(BigInt::from));
    }
}
