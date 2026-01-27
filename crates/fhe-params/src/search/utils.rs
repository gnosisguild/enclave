use num_bigint::BigUint;
use num_traits::{One, Zero};

pub fn parse_hex_big(s: &str) -> BigUint {
    let t = s.trim_start_matches("0x");
    BigUint::parse_bytes(t.as_bytes(), 16).expect("invalid hex prime")
}

pub fn product(xs: &[BigUint]) -> BigUint {
    let mut acc = BigUint::one();
    for x in xs {
        acc *= x;
    }
    acc
}

pub fn log2_big(x: &BigUint) -> f64 {
    if x.is_zero() {
        return f64::NEG_INFINITY;
    }
    let bytes = x.to_bytes_be();
    let leading = bytes[0];
    let lead_bits = 8 - leading.leading_zeros() as usize;
    let bits = (bytes.len() - 1) * 8 + lead_bits;

    // refine with up to 8 bytes
    let take = bytes.len().min(8);
    let mut top: u64 = 0;
    for &byte in bytes.iter().take(take) {
        top = (top << 8) | byte as u64;
    }
    let frac = (top as f64).log2();
    let adjust = (take * 8) as f64;
    (bits as f64 - adjust) + frac
}

pub fn approx_bits_from_log2(log2x: f64) -> u64 {
    if log2x <= 0.0 {
        1
    } else {
        log2x.floor() as u64 + 1
    }
}

pub fn fmt_big_summary(x: &BigUint) -> String {
    let bits = approx_bits_from_log2(log2_big(x));
    format!("≈ 2^{bits} ({bits} bits)")
}

pub fn big_shift_pow2(exp: u32) -> BigUint {
    BigUint::one() << exp
}
