use std::sha256;

// nb. this is a placeholder for the actual circuit.
fn main(input: [u8; 32]) -> pub [u8; 32] {
    let mut result = input;
    for _ in 0..100 {
        result = sha256::digest(result);
    }
    result
}
