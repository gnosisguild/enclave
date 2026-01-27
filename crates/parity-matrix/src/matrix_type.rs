// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Type-safe matrix types with dimension validation.

use crate::errors::{ParityMatrixError, ParityMatrixResult};
use num_bigint::BigUint;
use num_traits::Zero;
use serde::{Deserialize, Serialize};

/// A matrix with runtime-determined dimensions.
///
/// Use this when dimensions are not known at compile time (e.g., null space results).
/// Dimensions are validated at construction and runtime operations check consistency.
///
/// # Example
///
/// ```
/// use parity_matrix::DynamicMatrix;
/// use num_bigint::BigUint;
///
/// let data = vec![
///     vec![BigUint::from(1u32), BigUint::from(2u32)],
///     vec![BigUint::from(3u32), BigUint::from(4u32)],
/// ];
/// let matrix = DynamicMatrix::new(data).unwrap();
/// assert_eq!(matrix.rows(), 2);
/// assert_eq!(matrix.cols(), 2);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DynamicMatrix {
    data: Vec<Vec<BigUint>>,
    rows: usize,
    cols: usize,
}

impl DynamicMatrix {
    /// Creates a new dynamic matrix from data, validating dimensions.
    ///
    /// # Errors
    ///
    /// Returns an error if rows have inconsistent lengths.
    pub fn new(data: Vec<Vec<BigUint>>) -> ParityMatrixResult<Self> {
        if data.is_empty() {
            return Ok(Self {
                data,
                rows: 0,
                cols: 0,
            });
        }

        let rows = data.len();
        let cols = data[0].len();

        for (i, row) in data.iter().enumerate() {
            if row.len() != cols {
                return Err(ParityMatrixError::dimension_mismatch(
                    cols,
                    row.len(),
                    &format!("columns in row {}", i),
                ));
            }
        }

        Ok(Self { data, rows, cols })
    }

    /// Creates a zero matrix of the specified dimensions.
    pub fn zeros(rows: usize, cols: usize) -> Self {
        Self {
            data: vec![vec![BigUint::zero(); cols]; rows],
            rows,
            cols,
        }
    }

    /// Returns the number of rows.
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Returns the number of columns.
    pub fn cols(&self) -> usize {
        self.cols
    }

    /// Returns a reference to the underlying data.
    pub fn data(&self) -> &[Vec<BigUint>] {
        &self.data
    }

    /// Gets a reference to a specific element.
    ///
    /// # Panics
    ///
    /// Panics if indices are out of bounds.
    pub fn get(&self, row: usize, col: usize) -> &BigUint {
        &self.data[row][col]
    }
}

impl From<DynamicMatrix> for Vec<Vec<BigUint>> {
    fn from(matrix: DynamicMatrix) -> Self {
        matrix.data
    }
}

/// Trait for matrices that can be used in matrix operations.
pub trait MatrixLike {
    /// Returns the number of rows.
    fn rows(&self) -> usize;

    /// Returns the number of columns.
    fn cols(&self) -> usize;

    /// Returns a reference to the underlying data.
    fn data(&self) -> &[Vec<BigUint>];
}

impl MatrixLike for DynamicMatrix {
    fn rows(&self) -> usize {
        self.rows
    }

    fn cols(&self) -> usize {
        self.cols
    }

    fn data(&self) -> &[Vec<BigUint>] {
        &self.data
    }
}
