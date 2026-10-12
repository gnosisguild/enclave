// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use e3_polynomial::Polynomial;
use num_bigint::BigInt;

fn polynomial(length: usize) -> Polynomial {
    Polynomial::new(
        (0..length)
            .map(|i| BigInt::from(i as i64 - length as i64 / 2))
            .collect(),
    )
}

fn coefficient_arithmetic(c: &mut Criterion) {
    let mut group = c.benchmark_group("coefficient_arithmetic");
    for n in [512, 8192] {
        let a = polynomial(n);
        let modulus = BigInt::from(1000000007);
        let divisible = a.scalar_mul(&modulus);
        group.bench_function(format!("add/{n}"), |b| b.iter(|| black_box(a.add(&a))));
        group.bench_function(format!("div_exact/{n}"), |b| {
            b.iter(|| black_box(divisible.div_exact(&modulus).unwrap()))
        });
        group.bench_function(format!("reduce_center/{n}"), |b| {
            b.iter(|| {
                let mut p = a.clone();
                p.reduce(&modulus);
                p.center(&modulus);
                black_box(p)
            })
        });
    }
    group.finish();
}

fn convolution(c: &mut Criterion) {
    let mut group = c.benchmark_group("integer_convolution");
    for n in [64, 512] {
        let a = polynomial(n);
        let ternary = Polynomial::new((0..n).map(|i| BigInt::from((i % 3) as i64 - 1)).collect());
        group.bench_function(format!("degree_{n}"), |b| {
            b.iter(|| black_box(a.mul(&ternary)))
        });
    }
    group.finish();
}

criterion_group!(benches, coefficient_arithmetic, convolution);
criterion_main!(benches);
