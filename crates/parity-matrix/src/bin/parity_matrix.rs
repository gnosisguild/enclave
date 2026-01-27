// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! BFV Parameter Search CLI
//!
//! Standalone command-line tool for searching BFV parameters using NTT-friendly primes.

use clap::Parser;
use num_bigint::BigUint;
use num_traits::Zero;
use parity_matrix::math::evaluate_polynomial;
use parity_matrix::matrix::{
    build_generator_matrix, null_space, verify_parity_matrix, ParityMatrixConfig,
};
use parity_matrix::utils::{print_matrix, verify_null_space};

#[derive(Parser, Debug, Clone)]
#[command(
    version,
    about = "Generate the parity matrix for the linear subspace of Z_q^{n+1} consisting of polynomial evaluations of degree at most t at points 0, 1, ..., n."
)]
struct Args {
    /// Modulus q
    #[arg(long, default_value_t = BigUint::from(101u128))]
    q: BigUint,

    /// Number of points n
    #[arg(long, default_value_t = 10usize)]
    n: usize,

    /// Degree t of the polynomial
    #[arg(long, default_value_t = 4usize)]
    t: usize,

    /// Verbose per-candidate logging
    #[arg(long, default_value_t = false)]
    verbose: bool,
}

fn main() {
    let args = Args::parse();

    if args.verbose {
        println!(
            "== Parity Matrix for the linear subspace of Z_{{q}}^{} consisting of polynomial evaluations of degree at most {{t}} at points 0, 1, ..., {}. ==",
            args.n + 1,
            args.n
        );
        println!("Inputs: q={}  n={} t={}", args.q, args.n, args.t);
        println!("Constraint: t ≤ (n-1)/2\n");
    }

    println!("=== Parity Matrix Generator ===");
    println!(
        "Subspace: Evaluations of polynomials of degree ≤ {} at points 0, 1, ..., {}",
        args.t, args.n
    );
    println!(
        "Code dimension: {} (number of free coefficients: a_0, ..., a_t)",
        args.t + 1
    );
    println!("Code length: {} (number of evaluation points)", args.n + 1);
    println!(
        "Expected parity matrix size: {} x {}",
        args.n + 1 - (args.t + 1),
        args.n + 1
    );

    let config = ParityMatrixConfig {
        q: args.q.clone(),
        t: args.t,
        n: args.n,
    };

    // Build the generator matrix (this will validate the config and constraints)
    let g = match build_generator_matrix(config) {
        Ok(pm) => pm,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };
    print_matrix("Generator Matrix G", &g, &args.q);

    // Compute the parity (null space) matrix
    let h = match null_space(&g, &args.q) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("Error computing null space: {}", e);
            std::process::exit(1);
        }
    };

    if h.is_empty() {
        println!("Parity Matrix H: (empty - the subspace spans the entire space)");
    } else {
        print_matrix("Parity Matrix H", &h, &args.q);
    }

    // Verify correctness
    match verify_parity_matrix(&g, &h, &args.q) {
        Ok(true) => {
            println!("✓ Verification passed: H · G^T = 0 (mod q)");
        }
        Ok(false) => {
            println!("✗ Verification FAILED: H · G^T ≠ 0 (mod q)");
        }
        Err(e) => {
            eprintln!("Error during verification: {}", e);
            std::process::exit(1);
        }
    }

    if args.verbose {
        println!(
            "\n================================================================================"
        );

        // Show example: verify a random polynomial evaluation is in the null space
        println!("=== Example Verification ===");

        // Create a polynomial of degree t with coefficients [1, 2, ..., t+1]
        let num_coeffs = args.t + 1;
        let coeffs: Vec<BigUint> = (1..=num_coeffs)
            .map(|x| BigUint::from(x) % &args.q)
            .collect();

        // Build polynomial string representation
        let poly_str: String = coeffs
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let c_str = c.to_string();
                match i {
                    0 => c_str,
                    1 => format!("{c_str}·x"),
                    _ => format!("{c_str}·x^{i}"),
                }
            })
            .collect::<Vec<_>>()
            .join(" + ");

        println!(
            "Example polynomial F(x) of degree {} with coefficients (a_0, ..., a_{}): {:?}",
            args.t,
            args.t,
            coeffs.iter().map(|c| c.to_string()).collect::<Vec<_>>()
        );
        println!("F(x) = {}", poly_str);

        let eval_vec = evaluate_polynomial(&coeffs, args.n, &args.q);
        println!(
            "Evaluation vector v = [F(0), F(1), ..., F({})]: {:?}",
            args.n,
            eval_vec.iter().map(|v| v.to_string()).collect::<Vec<_>>()
        );

        verify_null_space(
            &h,
            &eval_vec,
            &args.q,
            "Evaluation vector is in the null space of H (as expected)",
        );

        // Second verification with random coefficients
        println!("=== Second Verification (Random Coefficients) ===");

        // Generate pseudo-random coefficients using system time as seed
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64;

        let mut rng_state = seed;
        let random_coeffs: Vec<BigUint> = (0..num_coeffs)
            .map(|_| {
                // Simple LCG: next = (a * current + c) mod m
                rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
                BigUint::from(rng_state % 1000) % &args.q // Random value mod q
            })
            .collect();

        // Build polynomial string representation for random poly
        let random_poly_str: String = random_coeffs
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let c_str = c.to_string();
                match i {
                    0 => c_str,
                    1 => format!("{c_str}·x"),
                    _ => format!("{c_str}·x^{i}"),
                }
            })
            .collect::<Vec<_>>()
            .join(" + ");

        println!(
            "Random polynomial G(x) of degree {} with coefficients (a_0, ..., a_{}): {:?}",
            args.t,
            args.t,
            random_coeffs
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
        );
        println!("G(x) = {}", random_poly_str);

        let random_eval_vec = evaluate_polynomial(&random_coeffs, args.n, &args.q);
        println!(
            "Evaluation vector v = [G(0), G(1), ..., G({})]: {:?}",
            args.n,
            random_eval_vec
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
        );

        verify_null_space(
            &h,
            &random_eval_vec,
            &args.q,
            "Random polynomial evaluation is also in the null space of H",
        );
    }
}
