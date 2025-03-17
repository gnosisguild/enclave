/// `N` is the degree of the cyclotomic polynomial defining the ring `Rq = Zq[X]/(X^N + 1)`.
pub const N: usize = 2048;
/// The coefficients of the polynomial `pk0is` and `pk1is` should exist in the interval `[-PK_BOUND, PK_BOUND]`.
pub const PK_BOUND: [u64; 1] = [2251799813160960];
/// The coefficients of the polynomial `pk1is` should exist in the interval `[-PK0_BOUND, PK0_BOUND]`.
/// The coefficients of the polynomial `e` should exist in the interval `[-E_BOUND, E_BOUND]` where `E_BOUND` is the upper bound of the gaussian distribution with 𝜎 = 3.2.
pub const E_BOUND: u64 = 19;
/// The coefficients of the polynomial `s` should exist in the interval `[-S_BOUND, S_BOUND]`.
pub const U_BOUND: u64 = 19;
/// The coefficients of the polynomials `r1is` should exist in the interval `[R1_LOW_BOUNDS[i], R1_UP_BOUNDS[i]]` where R1_LOW_BOUNDS is equal to $\frac{\frac{-(t - 1)}{2} \cdot |K_{0,i}| - (N+2) \cdot \frac{q_i - 1}{2} + B}{q_i}` and `R1_UP_BOUNDS[i]` is equal to `$\frac{\frac{(t - 1)}{2} \cdot |K_{0,i}| + (N+2) \cdot \frac{q_i - 1}{2} + B}{q_i}` .
pub const R1_LOW_BOUNDS: [i64; 1] = [-283578];
pub const R1_UP_BOUNDS: [u64; 1] = [283578];
/// The coefficients of the polynomials `r2is` should exist in the interval `[-R2_BOUND[i], R2_BOUND[i]]` where `R2_BOUND[i]` is equal to `(qi-1)/2`.
pub const R2_BOUNDS: [u64; 1] = [2251799813160960];
/// The coefficients of the polynomials `p1is` should exist in the interval `[-P1_BOUND[i], P1_BOUND[i]]` where `P1_BOUND[i]` is equal to (((qis[i] - 1) / 2) * (n + 2) + b ) / qis[i].
pub const P1_BOUNDS: [u64; 1] = [1024];
/// The coefficients of the polynomials `p2is` should exist in the interval `[-P2_BOUND[i], P2_BOUND[i]]` where `P2_BOUND[i]` is equal to (qis[i] - 1) / 2.
pub const P2_BOUNDS: [u64; 1] = [2251799813160960];
/// The coefficients of `k1` should exist in the interval `[K1_LOW_BOUND, K1_UP_BOUND]` where `K1_LOW_BOUND` is equal to `(-(t-1))/2` and K1_UP_BOUND` is equal to `(t-1)/2`.
pub const K1_LOW_BOUND: i64 = -516096;
pub const K1_UP_BOUND: u64 = 516096;
/// List of scalars `qis` such that `qis[i]` is the modulus of the i-th CRT basis of `q` (ciphertext space modulus).
pub const QIS: [&str; 1] = ["4503599626321921"];
/// List of scalars `k0is` such that `k0i[i]` is equal to the negative of the multiplicative inverses of t mod qi.
pub const K0IS: [&str; 1] = ["2465643709685619"];
