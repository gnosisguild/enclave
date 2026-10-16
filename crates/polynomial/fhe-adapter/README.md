# e3-polynomial-fhe

One-way conversion from fhe-math ring polynomials into the parent's exact-integer `Polynomial` /
`CrtPolynomial` types. This is a separate package in the root workspace, not a feature or nested
workspace of `e3-polynomial`.

## Conversion API

Import `e3_polynomial_fhe::FromFhePolynomial` and call `Polynomial::from_fhe_polynomial(&poly)` or
`CrtPolynomial::from_fhe_polynomial(&poly)`. Inputs implementing `ToPowerBasisPoly` include
power-basis, NTT, NTT-Shoup, and references to those types. Transformed representations are
converted before coefficients are read.

- `CrtPolynomial` retains separate modulus rows in their original order.
- `Polynomial` reconstructs nonnegative integers modulo the combined coefficient modulus Q.
- Both retain native ascending coefficient order and shape.

The conversion does **not** reverse, center, pack, or encode a circuit witness. Arithmetic in
[e3-polynomial](../README.md) uses descending coefficients, so consumers must explicitly cross that
ordering boundary before arithmetic. [e3-zk-helpers](../../zk-helpers/README.md) owns the
circuit-specific reversal/centering and witness encoding rather than this generic adapter.

In a workspace package, depend on both `e3-polynomial` and `e3-polynomial-fhe` with
`workspace = true`; import the trait from the latter and destination types from the former. The
[adapter implementation](src/lib.rs) contains the complete supported conversion surface.

Run `cargo test --locked -p e3-polynomial-fhe` from the repository root. Conversion tests do not
replace the affected consumer's witness/parity tests.
