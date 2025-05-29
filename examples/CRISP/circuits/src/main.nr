/// `N` is the degree of the cyclotomic polynomial defining the ring `Rq = Zq[X]/(X^N + 1)`.
pub global N: u32 = 128;
/// `L` is the dimension size of the polynomials.
pub global L: u32 = 2;
/// Polynomial structure
pub struct Polynomial<let N: u32> {
    // Coefficients of the polynomial (Biggest degree at the first index)
    pub coefficients: [Field; N],
}

/// Main function
fn main(
    pk0is: [Polynomial<N>; L],
    pk1is: [Polynomial<N>; L],
    ct0is: [Polynomial<N>; L],
    ct1is: [Polynomial<N>; L],
    u: Polynomial<N>,
    e0: Polynomial<N>,
    e1: Polynomial<N>,
    k1: Polynomial<N>,
    r1is: [Polynomial<2 * N - 1>; L],
    r2is: [Polynomial<N - 1>; L],
    p1is: [Polynomial<2 * N - 1>; L],
    p2is: [Polynomial<N - 1>; L],
) {
    assert(1 != 2);
}
