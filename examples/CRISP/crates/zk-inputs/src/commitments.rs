use ark_bn254::Fr as Field;
use ark_ff::BigInteger;
use ark_ff::PrimeField;
use eyre::Result;
use fhe::bfv::BfvParameters;
use greco::bounds::GrecoBounds;
use num_bigint::BigInt;
use shared::packing::flatten;
use shared::utils::compute_safe;
use std::sync::Arc;

/// Computes the commitment to a set of ciphertext polynomials.
///
/// # Arguments
/// * `ct0is` - The first component of the ciphertext polynomials.
/// * `ct1is` - The second component of the ciphertext polynomials.
///
/// # Returns
/// The commitment as a BigInt.
pub fn compute_commitment(
    bfv_params: Arc<BfvParameters>,
    ct0is: &[Vec<BigInt>],
    ct1is: &[Vec<BigInt>],
) -> Result<BigInt> {
    let (_, bounds) = GrecoBounds::compute(&bfv_params, 0)?;
    let bit = shared::template::calculate_bit_width(&bounds.pk_bounds[0].to_string())?;

    // Step 1: Flatten both polynomial components (matches commitment_payload in Noir)
    let mut inputs: Vec<Field> = Vec::new();
    inputs = flatten(inputs, ct0is, bit);
    inputs = flatten(inputs, ct1is, bit);

    // Step 2: Hash using SafeSponge (matches generate_challenge in Noir)
    // Domain separator - "CRISP_CT"
    let domain_separator: [u8; 64] = [
        0x43, 0x52, 0x49, 0x53, 0x50, 0x5f, 0x43, 0x54, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00,
    ];

    // IO Pattern: ABSORB(input_size), SQUEEZE(1)
    let input_size = inputs.len() as u32;
    let io_pattern = [0x80000000 | input_size, 1];

    let commitment = compute_safe(domain_separator, inputs, io_pattern);

    // Convert Field to BigInt
    let commitment_field = commitment[0];
    let commitment_bytes = commitment_field.into_bigint().to_bytes_le();

    Ok(BigInt::from_bytes_le(
        num_bigint::Sign::Plus,
        &commitment_bytes,
    ))
}
