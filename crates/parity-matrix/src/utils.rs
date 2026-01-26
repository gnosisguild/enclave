// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use num_bigint::BigUint;

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
