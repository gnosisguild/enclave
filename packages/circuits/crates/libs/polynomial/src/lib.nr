// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

/// Polynomial structure representing a polynomial of degree N-1.
///
/// A polynomial P(X) = a_{N-1} * X^{N-1} + a_{N-2} * X^{N-2} + ... + a_1 * X + a_0
/// is represented as an array of coefficients where coefficients[0] = a_{N-1} (highest degree)
/// and coefficients[N-1] = a_0 (constant term).
pub struct Polynomial<let N: u32> {
    /// Array of polynomial coefficients in descending degree order
    /// coefficients[0] = coefficient of X^{N-1} (highest degree term)
    /// coefficients[N-1] = constant term (degree 0)
    pub coefficients: [Field; N],
}

impl<let N: u32> Polynomial<N> {
    /// Creates a new polynomial from an array of coefficients.
    ///
    /// # Arguments
    /// * `coefficients` - Array of N coefficients in descending degree order
    ///                    coefficients[0] = coefficient of X^{N-1}
    ///                    coefficients[N-1] = constant term
    ///
    /// # Returns
    /// A new Polynomial instance with the specified coefficients
    pub fn new(coefficients: [Field; N]) -> Self {
        Polynomial { coefficients }
    }

    /// Evaluates the polynomial at a given point using Horner's method.
    ///
    /// Horner's method computes P(x) = a_{N-1} * x^{N-1} + ... + a_1 * x + a_0
    /// as ((...((a_{N-1} * x + a_{N-2}) * x + a_{N-3}) * x + ...) * x + a_0)
    /// This approach require n multiplications and n additions to evaluate the polynomial.
    ///
    /// # Arguments
    /// * `x` - The point at which to evaluate the polynomial.
    ///
    /// # Returns
    /// The value of the polynomial at point x: P(x).
    pub fn eval(self, x: Field) -> Field {
        let mut result = self.coefficients[0];

        for i in 1..self.coefficients.len() {
            result = result * x + self.coefficients[i];
        }

        result
    }

    /// Performs range checking on polynomial coefficients using a symmetric bound.
    ///
    /// This function constrains all polynomial coefficients to be in the range [-upper_bound, upper_bound].
    /// The implementation uses a clever shifting technique to avoid negative numbers in the range check:
    /// 1. Shifts each coefficient by adding upper_bound: c' = c + upper_bound
    /// 2. Checks that shifted coefficients are in [0, 2*upper_bound]
    /// 3. This ensures original coefficients are in [-upper_bound, upper_bound]
    ///
    /// # Arguments
    /// * `upper_bound` - The symmetric bound for coefficient range checking
    ///                   Coefficients must satisfy: -upper_bound <= c <= upper_bound
    pub fn range_check_1bound(self, upper_bound: u64) {
        let shifted_bound = (upper_bound as Field) + (upper_bound as Field);

        for i in 0..self.coefficients.len() {
            let shifted_coefficient = self.coefficients[i] + upper_bound as Field;

            shifted_coefficient.assert_max_bit_size::<66>();
            (shifted_bound - shifted_coefficient).assert_max_bit_size::<66>();
        }
    }

    /// Performs range checking on polynomial coefficients using asymmetric bounds
    ///
    /// This function constrains all polynomial coefficients to be in the range [lower_bound, upper_bound].
    /// Similar to range_check_1bound, it uses a shifting technique to avoid negative numbers:
    /// 1. Shifts each coefficient by adding -lower_bound: c' = c - lower_bound
    /// 2. Checks that shifted coefficients are in [0, upper_bound - lower_bound]
    /// 3. This ensures original coefficients are in [lower_bound, upper_bound]
    ///
    /// # Arguments
    /// * `upper_bound` - The upper bound for coefficient range checking
    /// * `lower_bound` - The lower bound for coefficient range checking (can be negative)
    ///                   Coefficients must satisfy: lower_bound <= c <= upper_bound
    pub fn range_check_2bounds(self, upper_bound: u64, lower_bound: i64) {
        let shifted_amount = (lower_bound * -1) as u64 as Field;
        let range_size = shifted_amount + upper_bound as Field;

        for i in 0..self.coefficients.len() {
            let shifted_coefficient = self.coefficients[i] + shifted_amount;

            shifted_coefficient.assert_max_bit_size::<66>();
            (range_size - shifted_coefficient).assert_max_bit_size::<66>();
        }
    }
}

/// Flatten `L` polynomials (each `Polynomial<A>`) into a linear Vec<Field> of packed carriers.
/// - Uses the same hex-aligned packing as `packer::<A, BIT>`:
///   `nibble_bits = ceil_to_nibble(BIT)`, `limb_bits = nibble_bits + 4`,
///   `group = 254 / limb_bits` carriers per chunk.
/// - Preserves deterministic order: iterate polynomials in `j`, then append all their carriers.
pub fn flatten<let A: u32, let L: u32, let BIT: u32>(
    mut inputs: Vec<Field>,
    poly: [Polynomial<A>; L],
) -> Vec<Field> {
    // Compute packing layout (must mirror `packer`).
    let nibble_bits = ((BIT + 3) / 4) * 4; // ceil to multiple of 4
    let limb_bits = nibble_bits + 4; // +1 nibble offset
    assert(limb_bits <= 254);

    let group = 254 / limb_bits; // max limbs per field
    assert(group >= 1);

    // Carriers produced per polynomial.
    let num_chunks = (A + group - 1) / group;

    // Pack each polynomial, then append all its carriers in order.
    for j in 0..L {
        let packed = packer::<A, BIT>(poly[j].coefficients);
        // `packed.len()` == `num_chunks`; push explicitly to be clear/stable.
        for i in 0..num_chunks {
            inputs.push(packed.get(i));
        }
    }

    inputs
}

/// Pack `values` into as few `Field` carriers as possible (hex-aligned).
/// - `BIT`: bit width of each value (before offset). We round it up to a nibble.
/// - Limb = `v + 2^nibble_bits` (lifts negatives); radix per limb = `2^(nibble_bits+4)`.
/// - Packs `group = 254 / (nibble_bits+4)` values per carrier, big-endian; pads tail with `2^nibble_bits`.
pub fn packer<let A: u32, let BIT: u32>(values: [Field; A]) -> Vec<Field> {
    assert(BIT >= 1);

    // Align to hex digits (4 bits) and add +1 nibble headroom.
    let nibble_bits = ((BIT + 3) / 4) * 4; // ceil_to_nibble(BIT)
    let limb_bits = nibble_bits + 4; // limb width used for packing
    assert(limb_bits <= 254);

    // Max limbs per 254-bit field element.
    let group = 254 / limb_bits;
    assert(group >= 1);

    // Offset and radix for limb concatenation.
    let base = 2.pow_32(nibble_bits as Field); // 2^nibble_bits
    let radix = 2.pow_32(limb_bits as Field); // 2^(nibble_bits+4)

    // Number of carriers needed.
    let num_chunks = (A + group - 1) / group;
    let mut out = Vec::new();

    // Process in fixed-size chunks of `group` limbs.
    for chunk in 0..num_chunks {
        // How many real values to take in this chunk.
        let remain = if A > chunk * group {
            A - chunk * group
        } else {
            0
        };
        let take = if remain < group { remain } else { group };

        // Build one carrier (big-endian): acc = acc * radix + (v + base)
        let mut acc = 0;
        for i in 0..take {
            let v = values[chunk * group + i];
            acc = acc * radix + (v + base);
        }

        // Pad remaining limbs with `base` (encodes v=0) to keep layout deterministic.
        for _ in 0..(group - take) {
            acc = acc * radix + base;
        }

        out.push(acc);
    }
    out
}

#[test]
fn test_polynomial_eval() {
    let coeffs = [1, 2, 3]; // represents 1x^2 + 2x + 3
    let poly = Polynomial::new(coeffs);

    let x = 2; // evaluate at x = 2
    let result = poly.eval(x);

    // (1 * 2^2) + (2 * 2) + 3 = 4 + 4 + 3 = 11
    assert_eq(result, 11);
}

#[test]
fn test_polynomial_eval_zero() {
    let coeffs = [1, -2, 1]; // x^2 - 2x + 1 = (x-1)^2
    let poly = Polynomial::new(coeffs);

    let x = 1; // evaluate at x = 1, should be 0
    let result = poly.eval(x);

    assert_eq(result, 0);
}

#[test]
fn test_polynomial_bounds() {
    let coeffs = [-16, 240, 242];
    let poly = Polynomial::new(coeffs);

    // Test single bound check - constrains to [-242, 242]
    poly.range_check_1bound(242);

    // Test double bounds check - constrains to [-16, 242]
    poly.range_check_2bounds(242, -16);
}

#[test(should_fail_with = "assert_max_bit_size")]
fn test_polynomial_out_of_bounds_coefficients() {
    let coeffs = [-100];
    let poly = Polynomial::new(coeffs);

    // Test single bound check - constrains to [-99, 99]
    // Should fail because -100 is out of bounds.
    poly.range_check_1bound(99);

    // Test double bounds check - constrains to [-9, 99]
    // Should fail because -100 is out of bounds.
    poly.range_check_2bounds(99, -9);
}

#[test]
fn test_flatten() {
    // Create test polynomials
    let poly1 = Polynomial::new([1, 2, 3]); // degree 2
    let poly2 = Polynomial::new([4, -16, 6]); // degree 2
    let poly3 = Polynomial::new([-7, 8, 9]); // degree 2

    let polynomials = [poly1, poly2, poly3];

    // Initialize target array with zeros
    let mut inputs = Vec::new();

    // Flatten the polynomials
    let result = flatten::<_, _, 4>(inputs, polynomials);

    // Verify the flattened coefficients are in the correct positions
    // Every value shifted 1 nibble incase of negative integers
    assert_eq(result.get(0), 0x11121310101010101010101010101010101010101010101010101010101010);
    assert_eq(result.get(1), 0x14001610101010101010101010101010101010101010101010101010101010); // -16 became 00 at  0x 14 00 16,
    assert_eq(result.get(2), 0x09181910101010101010101010101010101010101010101010101010101010); // -7 became 09 at 0x 09 18 19(16 - 7 = 9)
}

#[test]
fn test_flatten_big() {
    // Create test polynomials
    let poly1 = Polynomial::new([
        1791218451968394,
        21888242871839275222246405745257275088548364400416034343698198265248580087864,
        21888242871839275222246405745257275088548364400416034343698200542108324633466,
        5430119342984413,
        704811298945172,
        8901715723925099,
        21888242871839275222246405745257275088548364400416034343698203098124042812559,
        21888242871839275222246405745257275088548364400416034343698200215091693880034,
    ]);
    let poly2 = Polynomial::new([
        21888242871839275222246405745257275088548364400416034343698200314078269634250,
        21888242871839275222246405745257275088548364400416034343698200967285641915872,
        2909990636858607,
        7896103832076587,
        2078397209533893,
        21888242871839275222246405745257275088548364400416034343698199792421452734531,
        614400389245817,
        8290314119277588,
    ]);
    let poly3 = Polynomial::new([
        21888242871839275222246405745257275088548364400416034343698201373175279892906,
        21888242871839275222246405745257275088548364400416034343698201087241869723721,
        6768789983786188,
        635797784303388,
        7610153424227556,
        4633893206538324,
        2016269760615332,
        21888242871839275222246405745257275088548364400416034343698201007080554428142,
    ]);

    let polynomials = [poly1, poly2, poly3];

    // Initialize target array with zeros
    let mut inputs = Vec::new();

    // Flatten the polynomials
    let result = flatten::<_, _, 54>(inputs, polynomials);

    // Verify the flattened coefficients are in the correct positions
    // Every value shifted 1 nibble incase of negative integers

    // For the first index of result operation goes like this,

    // First four index of poly1
    // 1791218451968394,
    // 21888242871839275222246405745257275088548364400416034343698198265248580087864,
    // 21888242871839275222246405745257275088548364400416034343698200542108324633466,
    // 5430119342984413,

    // base + 1791218451968394 = 0x1065d1a8b8b718a
    // base - 5921327228407753 = 0xeaf69591f3b037 (negative coefficient shifted)
    // base - 3644467483862151 = 0xf30d604a3a9b79 (negative coefficient shifted)
    // base + 5430119342984413 = 0x1134aaa2e86ccdd
    assert_eq(result.get(0), 0x1065d1a8b8b718a0eaf69591f3b0370f30d604a3a9b791134aaa2e86ccdd);
    assert_eq(result.get(1), 0x1028105ab1b789411fa010339db66b0fc220f1326bc8e0f1e3f4cc1e02e1);
    assert_eq(result.get(2), 0x0f23dfbe7cd76c90f4901299312ddf10a569efe35acef11c0d76f005412b);
    assert_eq(result.get(3), 0x107624a8f605dc50f0638a368960421022ecb3cf36b7911d73ff2c27ec14);
    assert_eq(result.get(4), 0x0f6013a24e1b9a90f4fd2c158a08481180c2dba8af4cc10242413515171c);
    assert_eq(result.get(5), 0x11b0964eb898ce411076805680b85410729c962da53a40f4b44412d0f6ed);
}

#[test]
fn test_flatten_small() {
    // Create test polynomials
    let poly1 = Polynomial::new([712345, 104857, 999999, 500001, 123, 654321, 77]);
    let poly2 = Polynomial::new([1, 524287, 888888, 23456, 34567, 765432, 0]);
    let poly3 = Polynomial::new([444444, 333333, 222222, 111111, 987654, 246810, 13579]);

    let polynomials = [poly1, poly2, poly3];

    // Initialize target array with zeros
    let mut inputs = Vec::new();

    // Flatten the polynomials
    let result = flatten::<_, _, 20>(inputs, polynomials);

    assert_eq(result.get(0), 0x1ade991199991f423f17a12110007b19fbf110004d100000100000100000);
    assert_eq(result.get(1), 0x10000117ffff1d9038105ba01087071badf8100000100000100000100000);
    assert_eq(result.get(2), 0x16c81c15161513640e11b2071f120613c41a10350b100000100000100000);
}
