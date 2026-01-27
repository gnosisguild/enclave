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

    // Calculate max width and print in a single pass
    let mut max_width = q.to_string().len().min(6);
    let mut string_cache = Vec::new();

    for row in matrix {
        let mut row_strings = Vec::new();
        for val in row {
            let s = val.to_string();
            let display_len = if s.len() > 12 { 12 } else { s.len() };
            max_width = max_width.max(display_len);
            row_strings.push(s);
        }
        string_cache.push(row_strings);
    }

    for row_strings in string_cache {
        print!("  [");
        for (i, s) in row_strings.iter().enumerate() {
            if i > 0 {
                print!(", ");
            }
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
