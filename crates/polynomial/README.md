# Polynomial crates

This folder holds two workspace members:

- `core/` — the `e3-polynomial` crate. Polynomials with `num-bigint` coefficients, CRT limb
  containers, modular arithmetic and range checks, negacyclic folding, and coefficient
  representation helpers. This crate has no `fhe-math` dependency.
- `fhe-adapter/` — the `e3-polynomial-fhe` crate. Extension traits and functions that convert
  between the core types and the `fhe-math` RNS polynomial types (`Poly<PowerBasis>`,
  `Poly<Ntt>`, `Poly<NttShoup>`).

This folder is not a Cargo package and not a nested workspace. Both crates are direct members of
the root workspace. Depend on the crate that owns what you need:

- Only core polynomial arithmetic: depend on `e3-polynomial`.
- Conversions to or from FHE ciphertext polynomials: depend on `e3-polynomial-fhe` (it
  re-exports nothing from the core crate, so depend on both crates and import the
  `FromFhePolynomial`, `ToFhePolynomial`, and `ToPowerBasisPoly` traits).

## Testing

```bash
cargo test -p e3-polynomial
```

## Benchmarks

The core crate keeps the criterion benchmarks:

```bash
cargo bench -p e3-polynomial
```
