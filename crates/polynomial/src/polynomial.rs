// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Exact integer arithmetic on descending polynomial coefficients.

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{ToPrimitive, Zero};
use thiserror::Error;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Debug, Error)]
pub enum PolynomialError {
    #[error("Division by zero polynomial")]
    DivisionByZero,
    #[error("Invalid polynomial: {message}")]
    InvalidPolynomial { message: String },
}

/// Integer coefficients, without implicit modular reduction or normalization.
/// Arithmetic uses descending order. Constructors preserve leading zeros and length.
/// FHE coefficient views can retain ascending order until explicitly reversed.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct Polynomial {
    coefficients: Vec<BigInt>,
}

impl Polynomial {
    pub fn new(coefficients: Vec<BigInt>) -> Self {
        Self { coefficients }
    }

    pub fn from_u64_vector(coefficients: Vec<u64>) -> Self {
        Self::new(coefficients.into_iter().map(BigInt::from).collect())
    }

    pub fn coefficients(&self) -> &[BigInt] {
        &self.coefficients
    }

    /// Convert between ascending and descending coefficient order.
    pub fn reverse(&mut self) {
        self.coefficients.reverse();
    }

    pub fn is_zero(&self) -> bool {
        self.coefficients.iter().all(Zero::is_zero)
    }

    /// Add descending coefficients, aligning equal powers without trimming zeros.
    pub fn add(&self, other: &Self) -> Self {
        let len = self.coefficients.len().max(other.coefficients.len());
        let mut result = vec![BigInt::zero(); len];
        for (i, coefficient) in self.coefficients.iter().enumerate() {
            result[len - self.coefficients.len() + i] = coefficient.clone();
        }
        for (i, coefficient) in other.coefficients.iter().enumerate() {
            result[len - other.coefficients.len() + i] += coefficient;
        }
        Self::new(result)
    }

    pub fn sub(&self, other: &Self) -> Self {
        self.add(&other.neg())
    }

    pub fn neg(&self) -> Self {
        Self::new(self.coefficients.iter().map(|x| -x).collect())
    }

    /// Integer convolution, without reduction modulo a coefficient or ring modulus.
    /// A zero operand returns the single-coefficient zero polynomial.
    pub fn mul(&self, other: &Self) -> Self {
        if self.is_zero() || other.is_zero() {
            return Self::new(vec![BigInt::zero()]);
        }
        if let Some(product) = self.mul_i128(other) {
            return product;
        }
        let mut result =
            vec![BigInt::zero(); self.coefficients.len() + other.coefficients.len() - 1];
        for (i, a) in self.coefficients.iter().enumerate() {
            for (j, b) in other.coefficients.iter().enumerate() {
                result[i + j] += a * b;
            }
        }
        Self::new(result)
    }

    /// Use i128 only when every coefficient and partial convolution sum fits.
    fn mul_i128(&self, other: &Self) -> Option<Self> {
        let lhs: Vec<_> = self
            .coefficients
            .iter()
            .map(BigInt::to_i128)
            .collect::<Option<_>>()?;
        let rhs: Vec<_> = other
            .coefficients
            .iter()
            .map(BigInt::to_i128)
            .collect::<Option<_>>()?;
        let max_lhs = lhs.iter().map(|c| c.unsigned_abs()).max()?;
        let max_rhs = rhs.iter().map(|c| c.unsigned_abs()).max()?;
        let terms = lhs.len().min(rhs.len()) as u128;
        if max_lhs.checked_mul(max_rhs)?.checked_mul(terms)? > i128::MAX as u128 {
            return None;
        }
        let mut result = vec![0i128; lhs.len() + rhs.len() - 1];
        for (i, &a) in lhs.iter().enumerate() {
            if a != 0 {
                for (j, &b) in rhs.iter().enumerate() {
                    result[i + j] += a * b;
                }
            }
        }
        Some(Self::new(result.into_iter().map(BigInt::from).collect()))
    }

    /// Divide each coefficient exactly, preserving signs, order, and padding.
    /// Reject zero divisors and any coefficient with a nonzero remainder.
    pub fn div_exact(&self, divisor: &BigInt) -> Result<Self, PolynomialError> {
        if divisor.is_zero() {
            return Err(PolynomialError::DivisionByZero);
        }
        self.coefficients.iter().map(|coefficient| {
            let (quotient, remainder) = coefficient.div_rem(divisor);
            if !remainder.is_zero() {
                return Err(PolynomialError::InvalidPolynomial {
                    message: format!(
                        "Polynomial division requires exact divisibility: coefficient {} is not divisible by leading coefficient {}",
                        coefficient, divisor
                    ),
                });
            }
            Ok(quotient)
        }).collect::<Result<Vec<_>, _>>().map(Self::new)
    }

    pub fn scalar_mul(&self, scalar: &BigInt) -> Self {
        Self::new(self.coefficients.iter().map(|x| x * scalar).collect())
    }

    /// Center coefficients that are already in `[0, modulus)`.
    pub fn center(&mut self, modulus: &BigInt) {
        let midpoint = (modulus - 1u32) / 2u32;
        for coefficient in &mut self.coefficients {
            if *coefficient > midpoint {
                *coefficient -= modulus;
            }
        }
    }

    /// Reduce coefficients into `[0, modulus)`.
    pub fn reduce(&mut self, modulus: &BigInt) {
        for coefficient in &mut self.coefficients {
            *coefficient = coefficient.mod_floor(modulus);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn poly(coefficients: &[i64]) -> Polynomial {
        Polynomial::new(coefficients.iter().copied().map(BigInt::from).collect())
    }

    #[test]
    fn arithmetic_preserves_descending_alignment_and_padding() {
        let a = poly(&[0, 1, -2, 3]);
        let b = poly(&[4, -5]);
        assert_eq!(a.add(&b), poly(&[0, 1, 2, -2]));
        assert_eq!(a.sub(&b), poly(&[0, 1, -6, 8]));
        assert_eq!(a.sub(&a), poly(&[0, 0, 0, 0]));
        assert_eq!(a.scalar_mul(&BigInt::from(-2)), poly(&[0, -2, 4, -6]));
        assert_eq!(poly(&[0, 1, 2]).mul(&poly(&[1, 3])), poly(&[0, 1, 5, 6]));
        assert_eq!(a.mul(&poly(&[0, 0])), poly(&[0]));
    }

    #[test]
    fn signed_residues_preserve_centering_boundaries_and_padding() {
        for (coefficients, modulus, residues, centered) in [
            (
                vec![0, 10, -3, 6, -10, -14, -15, -21, -7, -6, -1, 3, 7],
                7,
                vec![0, 3, 4, 6, 4, 0, 6, 0, 0, 1, 6, 3, 0],
                vec![0, 3, -3, -1, -3, 0, -1, 0, 0, 1, -1, 3, 0],
            ),
            (vec![0, 2, 3, -1], 6, vec![0, 2, 3, 5], vec![0, 2, -3, -1]),
            (vec![0, 2, 3], 5, vec![0, 2, 3], vec![0, 2, -2]),
        ] {
            let mut value = poly(&coefficients);
            let modulus = BigInt::from(modulus);
            value.reduce(&modulus);
            assert_eq!(value, poly(&residues));
            value.center(&modulus);
            assert_eq!(value, poly(&centered));
        }
    }

    #[test]
    fn exact_scalar_division_preserves_shape_and_rejects_remainders() {
        let numerator = poly(&[0, -194, 97, 0]);
        assert_eq!(
            numerator.div_exact(&BigInt::from(97)).unwrap(),
            poly(&[0, -2, 1, 0])
        );
        assert_eq!(
            numerator.div_exact(&BigInt::from(-97)).unwrap(),
            poly(&[0, 2, -1, 0])
        );
        assert_eq!(
            poly(&[0, 0]).div_exact(&BigInt::from(97)).unwrap(),
            poly(&[0, 0])
        );
        assert_eq!(poly(&[]).div_exact(&BigInt::from(97)).unwrap(), poly(&[]));
        assert!(matches!(
            numerator.div_exact(&BigInt::zero()),
            Err(PolynomialError::DivisionByZero)
        ));
        assert!(matches!(
            poly(&[97, 98]).div_exact(&BigInt::from(97)),
            Err(PolynomialError::InvalidPolynomial { .. })
        ));
    }

    /// Independent arbitrary-precision convolution checks the bounded i128 path.
    fn schoolbook(lhs: &Polynomial, rhs: &Polynomial) -> Polynomial {
        let mut product = vec![BigInt::zero(); lhs.coefficients.len() + rhs.coefficients.len() - 1];
        for (i, a) in lhs.coefficients.iter().enumerate() {
            for (j, b) in rhs.coefficients.iter().enumerate() {
                product[i + j] += a * b;
            }
        }
        Polynomial::new(product)
    }

    #[test]
    fn multiplication_matches_integer_reference_for_witness_coefficients() {
        let mut seed = 0x2545F4914F6CDD1Du64;
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let qi = 0x02000000015a0001u64;
        let centered = Polynomial::new(
            (0..256)
                .map(|_| BigInt::from((next() % qi) as i128 - (qi / 2) as i128))
                .collect(),
        );
        let ternary = Polynomial::new((0..256).map(|_| BigInt::from(next() % 3) - 1).collect());
        assert!(centered.mul_i128(&ternary).is_some());
        assert_eq!(centered.mul(&ternary), schoolbook(&centered, &ternary));
    }

    #[test]
    fn multiplication_keeps_arbitrary_precision_when_i128_is_insufficient() {
        for coefficients in [
            vec![BigInt::from(1) << 100, BigInt::from(-1) << 100],
            vec![BigInt::from(1) << 200, BigInt::from(3)],
        ] {
            let p = Polynomial::new(coefficients);
            assert!(p.mul_i128(&p).is_none());
            assert_eq!(p.mul(&p), schoolbook(&p, &p));
        }
    }
}
