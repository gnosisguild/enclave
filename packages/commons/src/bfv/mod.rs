use anyhow::Context;
use anyhow::Result;
use fhe_rs::bfv::{BfvParameters, BfvParametersBuilder};
use fhe_traits::{Deserialize, Serialize};
use std::sync::Arc;

/// Predefined BFV parameters for common use cases
pub mod params {
    /// Standard BFV parameters sets
    /// Each set is a tuple of (degree, plaintext_modulus, moduli).
    /// Naming convention: SET_<degree>_<plaintext_modulus>_<moduli_count>

    /// - Degree: 2048 (polynomial ring size)
    /// - Plaintext modulus: 1032193
    /// - Moduli: [0x3FFFFFFF000001] (provides good security level)
    pub const SET_2048_1032193_1: (usize, u64, [u64; 1]) = (
        2048,              // degree
        1032193,           // plaintext_modulus
        [0x3FFFFFFF000001], // moduli
    );
}

/// Builds BFV (Brakerski-Fan-Vercauteren) encryption parameters.
///
/// # Arguments
///
/// * `degree` - The degree of the polynomial ring, must be a power of 2
/// * `plaintext_modulus` - The modulus for the plaintext space
/// * `moduli` - The moduli for the ciphertext space
///
/// # Returns
///
/// Returns a `BfvParameters` instance configured with the specified parameters.
///
/// # Panics
///
/// Panics if the parameters cannot be built (e.g., invalid degree or moduli).
pub fn build_bfv_params(degree: usize, plaintext_modulus: u64, moduli: &[u64]) -> BfvParameters {
    match BfvParametersBuilder::new()
        .set_degree(degree)
        .set_plaintext_modulus(plaintext_modulus)
        .set_moduli(moduli)
        .build()
    {
        Ok(params) => params,
        Err(e) => panic!("Failed to build BFV Parameters: {}", e),
    }
}

/// Builds BFV encryption parameters wrapped in an `Arc` for shared ownership.
///
/// This function is similar to `build_bfv_params` but returns the parameters
/// wrapped in an `Arc` for thread-safe shared ownership.
///
/// # Arguments
///
/// * `degree` - The degree of the polynomial ring, must be a power of 2
/// * `plaintext_modulus` - The modulus for the plaintext space
/// * `moduli` - The moduli for the ciphertext space
///
/// # Returns
///
/// Returns an `Arc<BfvParameters>` instance configured with the specified parameters.
///
/// # Panics
///
/// Panics if the parameters cannot be built (e.g., invalid degree or moduli).
pub fn build_bfv_params_arc(
    degree: usize,
    plaintext_modulus: u64,
    moduli: &[u64],
) -> Arc<BfvParameters> {
    match BfvParametersBuilder::new()
        .set_degree(degree)
        .set_plaintext_modulus(plaintext_modulus)
        .set_moduli(moduli)
        .build_arc()
    {
        Ok(params) => params,
        Err(e) => panic!("Failed to build BFV Parameters wrapped in Arc: {}", e),
    }
}

/// Deserializes BFV parameters from a byte slice.
///
/// # Arguments
///
/// * `bytes` - The byte slice containing the serialized parameters
///
/// # Returns
///
/// Returns a `BfvParameters` instance deserialized from the bytes.
///
/// # Panics
///
/// Panics if the deserialization fails.
pub fn deserialize_bfv_params(bytes: &[u8]) -> BfvParameters {
    match BfvParameters::try_deserialize(bytes) {
        Ok(params) => params,
        Err(e) => panic!("Failed to deserialize BFV Parameters: {}", e),
    }
}

/// Deserializes BFV parameters from a byte slice and wraps them in an `Arc`.
///
/// This is a convenience function that combines `deserialize_bfv_params` with `Arc::new`
/// to provide thread-safe shared ownership of the deserialized parameters.
///
/// # Arguments
///
/// * `bytes` - The byte slice containing the serialized parameters
///
/// # Returns
///
/// Returns an `Arc<BfvParameters>` instance deserialized from the bytes.
///
/// # Panics
///
/// Panics if the deserialization fails (see `deserialize_bfv_params`).
pub fn deserialize_bfv_params_arc(bytes: &[u8]) -> Arc<BfvParameters> {
    Arc::new(deserialize_bfv_params(bytes))
}

/// Serializes BFV parameters into a byte vector.
///
/// # Arguments
///
/// * `params` - The BFV parameters to serialize
///
/// # Returns
///
/// Returns a `Vec<u8>` containing the serialized parameters.
///
/// # Panics
///
/// Panics if the serialization fails.
pub fn serialize_bfv_params(params: &BfvParameters) -> Vec<u8> {
    params.to_bytes()
}

/// Encodes BFV parameters into a byte vector.
///
/// This function takes a `BfvParameters` instance and returns it serialized as a byte vector.
///
/// # Arguments
///
/// * `params` - The BFV parameters to encode
///
/// # Returns
///
/// Returns a `Vec<u8>` containing the serialized parameters.
pub fn encode_bfv_params(params: &BfvParameters) -> Vec<u8> {
    params.to_bytes()
}

/// Decodes BFV parameters from a byte slice.
///
/// This function attempts to deserialize BFV parameters from a byte slice
/// and wraps them in an `Arc` for thread-safe shared ownership.
///
/// # Arguments
///
/// * `bytes` - The byte slice containing the serialized parameters
///
/// # Returns
///
/// Returns a `Result<Arc<BfvParameters>>` containing the deserialized parameters
/// or an error if deserialization fails.
pub fn decode_bfv_params(bytes: &[u8]) -> Result<Arc<BfvParameters>> {
    Ok(Arc::new(
        BfvParameters::try_deserialize(bytes).context("Could not decode Bfv Params")?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_bfv_params() {
        let degree = 2048;
        let plaintext_modulus = 1032193;
        let moduli = [0x3FFFFFFF000001];

        let params = build_bfv_params(degree, plaintext_modulus, &moduli);
        assert_eq!(params.degree(), degree);
        assert_eq!(params.plaintext(), plaintext_modulus);
        assert_eq!(params.moduli(), moduli);
    }

    #[test]
    fn test_build_bfv_params_arc() {
        let degree = 2048;
        let plaintext_modulus = 1032193;
        let moduli = [0x3FFFFFFF000001];

        let params = build_bfv_params_arc(degree, plaintext_modulus, &moduli);
        assert_eq!(params.degree(), degree);
        assert_eq!(params.plaintext(), plaintext_modulus);
        assert_eq!(params.moduli(), moduli);
    }

    #[test]
    fn test_deserialize_bfv_params() {
        let degree = 2048;
        let plaintext_modulus = 1032193;
        let moduli = [0x3FFFFFFF000001];

        let params = build_bfv_params(degree, plaintext_modulus, &moduli);
        let serialized = params.to_bytes();
        let deserialized = deserialize_bfv_params(&serialized);

        assert_eq!(deserialized.degree(), degree);
        assert_eq!(deserialized.plaintext(), plaintext_modulus);
        assert_eq!(deserialized.moduli(), moduli);
    }

    #[test]
    fn test_deserialize_bfv_params_arc() {
        let degree = 2048;
        let plaintext_modulus = 1032193;
        let moduli = [0x3FFFFFFF000001];

        let params = build_bfv_params(degree, plaintext_modulus, &moduli);
        let serialized = params.to_bytes();
        let deserialized = deserialize_bfv_params_arc(&serialized);

        assert_eq!(deserialized.degree(), degree);
        assert_eq!(deserialized.plaintext(), plaintext_modulus);
        assert_eq!(deserialized.moduli(), moduli);
    }

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let degree = 2048;
        let plaintext_modulus = 1032193;
        let moduli = [0x3FFFFFFF000001];

        let params = build_bfv_params(degree, plaintext_modulus, &moduli);
        let serialized = serialize_bfv_params(&params);
        let deserialized = deserialize_bfv_params(&serialized);

        assert_eq!(deserialized.degree(), degree);
        assert_eq!(deserialized.plaintext(), plaintext_modulus);
        assert_eq!(deserialized.moduli(), moduli);
    }

    #[test]
    fn test_serialize_deserialize_arc_roundtrip() {
        let degree = 2048;
        let plaintext_modulus = 1032193;
        let moduli = [0x3FFFFFFF000001];

        let params = build_bfv_params_arc(degree, plaintext_modulus, &moduli);
        let serialized = serialize_bfv_params(&params);
        let deserialized = deserialize_bfv_params_arc(&serialized);

        assert_eq!(deserialized.degree(), degree);
        assert_eq!(deserialized.plaintext(), plaintext_modulus);
        assert_eq!(deserialized.moduli(), moduli);
    }

    #[test]
    fn test_encode_bfv_params() {
        let degree = 2048;
        let plaintext_modulus = 1032193;
        let moduli = vec![0x3FFFFFFF000001];

        let params = build_bfv_params(degree, plaintext_modulus, &moduli);
        let encoded = encode_bfv_params(&params);
        let decoded = decode_bfv_params(&encoded).unwrap();

        assert_eq!(decoded.degree(), degree);
        assert_eq!(decoded.plaintext(), plaintext_modulus);
        assert_eq!(decoded.moduli(), moduli.as_slice());
    }

    #[test]
    fn test_decode_params_error() {
        let invalid_bytes = vec![0u8; 10];
        let result = decode_bfv_params(&invalid_bytes);
        assert!(result.is_err());
    }

    mod params_tests {
        use super::*;

        #[test]
        fn test_params_constant() {
            let (degree, plaintext_modulus, moduli) = params::SET_2048_1032193_1;
            assert_eq!(degree, 2048);
            assert_eq!(plaintext_modulus, 1032193);
            assert_eq!(moduli, [0x3FFFFFFF000001]);
        }

        #[test]
        fn test_params_function() {
            let (degree, plaintext_modulus, moduli) = params::SET_2048_1032193_1;
            let params = build_bfv_params(degree, plaintext_modulus, &moduli);

            assert_eq!(params.degree(), degree);
            assert_eq!(params.plaintext(), plaintext_modulus);
            assert_eq!(params.moduli(), moduli);
        }

        #[test]
        fn test_params_arc_function() {
            let (degree, plaintext_modulus, moduli) = params::SET_2048_1032193_1;
            let params = build_bfv_params_arc(degree, plaintext_modulus, &moduli);

            assert_eq!(params.degree(), degree);
            assert_eq!(params.plaintext(), plaintext_modulus);
            assert_eq!(params.moduli(), moduli);
        }

        #[test]
        fn test_params_serialization_roundtrip() {
            let (degree, plaintext_modulus, moduli) = params::SET_2048_1032193_1;
            let params = build_bfv_params(degree, plaintext_modulus, &moduli);
            let serialized = serialize_bfv_params(&params);
            let deserialized = deserialize_bfv_params(&serialized);

            let (degree, plaintext_modulus, moduli) = params::SET_2048_1032193_1;
            assert_eq!(deserialized.degree(), degree);
            assert_eq!(deserialized.plaintext(), plaintext_modulus);
            assert_eq!(deserialized.moduli(), moduli);
        }

        #[test]
        fn test_params_arc_serialization_roundtrip() {
            let (degree, plaintext_modulus, moduli) = params::SET_2048_1032193_1;
            let params = build_bfv_params_arc(degree, plaintext_modulus, &moduli);
            let serialized = serialize_bfv_params(&params);
            let deserialized = deserialize_bfv_params_arc(&serialized);

            let (degree, plaintext_modulus, moduli) = params::SET_2048_1032193_1;
            assert_eq!(deserialized.degree(), degree);
            assert_eq!(deserialized.plaintext(), plaintext_modulus);
            assert_eq!(deserialized.moduli(), moduli);
        }
    }
}
