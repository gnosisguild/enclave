// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use keccak256::keccak256;

/// Given a public key, signature, and hashed message, verify the signature
pub fn verify_signature(
    hashed_message: [u8; 32],
    pub_key_x: [u8; 32],
    pub_key_y: [u8; 32],
    signature: [u8; 64],
) {
    let valid_signature =
        std::ecdsa_secp256k1::verify_signature(pub_key_x, pub_key_y, signature, hashed_message);
    assert(valid_signature);
}

/// Given a public key, derive the Ethereum address
pub fn derive_address(pub_key_x: [u8; 32], pub_key_y: [u8; 32]) -> [u8; 20] {
    let mut pub_key_bytes = [0; 64];
    for i in 0..32 {
        pub_key_bytes[i] = pub_key_x[i];
        pub_key_bytes[i + 32] = pub_key_y[i];
    }

    // Hash the concatenated public key
    let hash = keccak256(pub_key_bytes, 64);

    // Extract last 20 bytes as the Ethereum address
    let mut derived_address = [0; 20];
    for i in 0..20 {
        derived_address[i] = hash[i + 12];
    }

    derived_address
}

// Convert a 20-byte address to a field element.
pub fn address_to_field(addr: [u8; 20]) -> Field {
    let mut acc: Field = 0;

    for i in 0..20 {
        acc = acc * 256 + (addr[i] as Field);
    }

    acc
}

#[test]
fn test_derive_address() {
    let pub_key_x = [
        131, 24, 83, 91, 84, 16, 93, 74, 122, 174, 96, 192, 143, 196, 95, 150, 135, 24, 27, 79, 223,
        198, 37, 189, 26, 117, 63, 167, 57, 127, 237, 117,
    ];
    let pub_key_y = [
        53, 71, 241, 28, 168, 105, 102, 70, 242, 243, 172, 176, 142, 49, 1, 106, 250, 194, 62, 99,
        12, 93, 17, 245, 159, 97, 254, 245, 123, 13, 42, 165,
    ];
    let address = derive_address(pub_key_x, pub_key_y);
    assert(
        address
            == [
                243, 159, 214, 229, 26, 173, 136, 246, 244, 206, 106, 184, 130, 114, 121, 207, 255,
                185, 34, 102,
            ],
    );
}

#[test]
fn test_verify_signature() {
    let hashed_message = [
        67, 126, 157, 164, 162, 165, 56, 242, 155, 214, 113, 196, 83, 198, 228, 36, 174, 104, 152,
        87, 167, 108, 64, 34, 234, 161, 122, 55, 44, 62, 151, 55,
    ];
    let pub_key_x = [
        131, 24, 83, 91, 84, 16, 93, 74, 122, 174, 96, 192, 143, 196, 95, 150, 135, 24, 27, 79, 223,
        198, 37, 189, 26, 117, 63, 167, 57, 127, 237, 117,
    ];
    let pub_key_y = [
        53, 71, 241, 28, 168, 105, 102, 70, 242, 243, 172, 176, 142, 49, 1, 106, 250, 194, 62, 99,
        12, 93, 17, 245, 159, 97, 254, 245, 123, 13, 42, 165,
    ];
    let signature = [
        78, 219, 63, 96, 214, 92, 16, 36, 108, 207, 133, 108, 3, 248, 122, 23, 192, 236, 210, 253,
        206, 125, 83, 124, 163, 54, 106, 125, 188, 223, 192, 39, 44, 76, 228, 107, 188, 221, 183,
        124, 118, 143, 228, 126, 216, 173, 160, 231, 62, 52, 188, 154, 110, 230, 183, 71, 36, 161,
        171, 163, 213, 62, 223, 152,
    ];
    verify_signature(hashed_message, pub_key_x, pub_key_y, signature);
}

#[test]
fn test_verify_signature_sdk_input() {
    let hashed_message = [
        200, 232, 98, 162, 80, 131, 242, 57, 252, 76, 226, 45, 127, 206, 207, 39, 206, 44, 211, 171,
        113, 67, 121, 68, 78, 253, 202, 79, 29, 128, 130, 76,
    ];

    let pub_key_x = [
        131, 24, 83, 91, 84, 16, 93, 74, 122, 174, 96, 192, 143, 196, 95, 150, 135, 24, 27, 79, 223,
        198, 37, 189, 26, 117, 63, 167, 57, 127, 237, 117,
    ];
    let pub_key_y = [
        53, 71, 241, 28, 168, 105, 102, 70, 242, 243, 172, 176, 142, 49, 1, 106, 250, 194, 62, 99,
        12, 93, 17, 245, 159, 97, 254, 245, 123, 13, 42, 165,
    ];
    let signature = [
        22, 65, 67, 29, 14, 211, 253, 134, 129, 79, 2, 109, 166, 46, 17, 67, 75, 83, 198, 168, 81,
        98, 254, 167, 249, 146, 24, 191, 60, 48, 125, 236, 127, 54, 28, 35, 95, 7, 182, 88, 120, 10,
        253, 145, 165, 201, 214, 141, 106, 75, 20, 213, 235, 5, 17, 246, 104, 141, 62, 145, 20, 14,
        236, 18,
    ];

    verify_signature(hashed_message, pub_key_x, pub_key_y, signature);
}

#[test]
fn test_fail_verify_signature() {
    let hashed_message = [
        67, 126, 157, 164, 162, 165, 56, 242, 155, 214, 113, 196, 83, 198, 228, 36, 174, 104, 152,
        87, 167, 108, 64, 34, 234, 161, 122, 55, 44, 62, 151, 55,
    ];
    let pub_key_x = [
        131, 24, 83, 91, 84, 16, 93, 74, 122, 174, 96, 192, 143, 196, 95, 150, 135, 24, 27, 79, 223,
        198, 37, 189, 26, 117, 63, 167, 57, 127, 237, 117,
    ];
    let pub_key_y = [
        53, 71, 241, 28, 168, 105, 102, 70, 242, 243, 172, 176, 142, 49, 1, 106, 250, 194, 62, 99,
        12, 93, 17, 245, 159, 97, 254, 245, 123, 13, 42, 165,
    ];
    let signature = [
        78, 219, 63, 96, 214, 92, 16, 36, 108, 207, 133, 108, 3, 248, 122, 23, 192, 236, 210, 253,
        206, 125, 83, 124, 163, 54, 106, 125, 188, 223, 192, 39, 44, 76, 228, 107, 188, 221, 183,
        124, 118, 143, 228, 126, 216, 173, 160, 231, 62, 52, 188, 154, 110, 230, 183, 71, 36, 161,
        171, 163, 213, 62, 223, 151,
    ];
    let valid_signature =
        std::ecdsa_secp256k1::verify_signature(pub_key_x, pub_key_y, signature, hashed_message);
    assert(!valid_signature);
}

#[test]
fn test_address_to_field() {
    let address: [u8; 20] = [
        243, 159, 214, 229, 26, 173, 136, 246, 244, 206, 106, 184, 130, 114, 121, 207, 255, 185, 34,
        102,
    ];
    let field = address_to_field(address);

    assert(field == 0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266);
}
