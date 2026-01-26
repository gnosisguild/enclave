//! BFV Parameter Search CLI
//!
//! Standalone command-line tool for searching BFV parameters using NTT-friendly primes.

use clap::Parser;
use num_bigint::BigUint;
use num_traits::Zero;
use zkfhe_parity_matrix::math::mod_pow;
use zkfhe_parity_matrix::matrix::{
    build_generator_matrix, null_space, verify_parity_matrix, ParityMatrixConfig,
};
use zkfhe_parity_matrix::utils::print_matrix;

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

    // Check constraint: t ≤ (n-1)/2
    let max_t = (args.n.saturating_sub(1)) / 2;
    if args.t > max_t {
        eprintln!(
            "Error: t ({}) must be ≤ (n-1)/2 = {} for n = {}",
            args.t, max_t, args.n
        );
        std::process::exit(1);
    }

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

        // Evaluate at all points
        let mut eval_vec = vec![BigUint::zero(); args.n + 1];
        #[allow(clippy::needless_range_loop)]
        for j in 0..=args.n {
            let x = BigUint::from(j);
            let mut val = BigUint::zero();
            for (i, coeff) in coeffs.iter().enumerate() {
                val = (val + coeff * mod_pow(&x, i, &args.q)) % &args.q;
            }
            eval_vec[j] = val;
        }
        println!(
            "Evaluation vector v = [F(0), F(1), ..., F({})]: {:?}",
            args.n,
            eval_vec.iter().map(|v| v.to_string()).collect::<Vec<_>>()
        );

        // Check H * v = 0
        if !h.is_empty() {
            let mut result = vec![BigUint::zero(); h.len()];
            for (i, row) in h.iter().enumerate() {
                for (j, h_val) in row.iter().enumerate() {
                    result[i] = (&result[i] + h_val * &eval_vec[j]) % &args.q;
                }
            }
            println!(
                "H · v = {:?}",
                result.iter().map(|v| v.to_string()).collect::<Vec<_>>()
            );
            if result.iter().all(|x| x.is_zero()) {
                println!("✓ Evaluation vector is in the null space of H (as expected)");
            } else {
                println!("✗ Something went wrong - vector should be in null space");
            }
        }

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

        // Evaluate random polynomial at all points
        let mut random_eval_vec = vec![BigUint::zero(); args.n + 1];
        #[allow(clippy::needless_range_loop)]
        for j in 0..=args.n {
            let x = BigUint::from(j);
            let mut val = BigUint::zero();
            for (i, coeff) in random_coeffs.iter().enumerate() {
                val = (val + coeff * mod_pow(&x, i, &args.q)) % &args.q;
            }
            random_eval_vec[j] = val;
        }
        println!(
            "Evaluation vector v = [G(0), G(1), ..., G({})]: {:?}",
            args.n,
            random_eval_vec
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
        );

        // Check H * v = 0 for random polynomial
        if !h.is_empty() {
            let mut result = vec![BigUint::zero(); h.len()];
            for (i, row) in h.iter().enumerate() {
                for (j, h_val) in row.iter().enumerate() {
                    result[i] = (&result[i] + h_val * &random_eval_vec[j]) % &args.q;
                }
            }
            println!(
                "H · v = {:?}",
                result.iter().map(|v| v.to_string()).collect::<Vec<_>>()
            );
            if result.iter().all(|x| x.is_zero()) {
                println!("✓ Random polynomial evaluation is also in the null space of H");
            } else {
                println!("✗ Something went wrong - vector should be in null space");
            }
        }
    }
}
