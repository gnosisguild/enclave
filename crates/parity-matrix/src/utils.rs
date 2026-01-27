// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use num_bigint::BigUint;
use num_traits::Zero;

pub fn print_matrix(name: &str, matrix: &[Vec<BigUint>], q: &BigUint) {
    let cols = if matrix.is_empty() {
        0
    } else {
        matrix[0].len()
    };
    println!("{} ({}x{}):", name, matrix.len(), cols);

    // Determine max width for formatting
    let max_width = matrix
        .iter()
        .flat_map(|row| row.iter())
        .map(|v| v.to_string().len())
        .max()
        .unwrap_or(1)
        .max(q.to_string().len().min(6)); // Cap at 6 for very large q

    for row in matrix {
        print!("  [");
        for (i, val) in row.iter().enumerate() {
            if i > 0 {
                print!(", ");
            }
            let s = val.to_string();
            // Truncate very large numbers for display
            if s.len() > 12 {
                print!("{}...", &s[..9]);
            } else {
                print!("{:>width$}", s, width = max_width);
            }
        }
        println!("]");
    }
    println!();
}

pub fn verify_null_space(h: &[Vec<BigUint>], eval_vec: &[BigUint], q: &BigUint, success_msg: &str) {
    if h.is_empty() {
        return;
    }
    let mut result = vec![BigUint::zero(); h.len()];
    for (i, row) in h.iter().enumerate() {
        for (j, h_val) in row.iter().enumerate() {
            result[i] = (&result[i] + h_val * &eval_vec[j]) % q;
        }
    }
    println!(
        "H · v = {:?}",
        result.iter().map(|v| v.to_string()).collect::<Vec<_>>()
    );
    if result.iter().all(|x| x.is_zero()) {
        println!("✓ {}", success_msg);
    } else {
        println!("✗ Something went wrong - vector should be in null space");
    }
}
