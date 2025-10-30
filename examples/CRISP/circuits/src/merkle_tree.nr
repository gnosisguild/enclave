// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use binary_merkle_root::binary_merkle_root;
use poseidon::poseidon::bn254::hash_2 as poseidon;

// Get the Merkle root of a Merkle tree.
pub fn get_merkle_root(
    address: Field,
    balance: Field,
    depth: u32,
    indices: [u1; 20],
    siblings: [Field; 20],
) -> Field {
    // Calculate poseidon hash of address and balance.
    let leaf = poseidon([address, balance]);

    // Calculate Merkle root.
    let merkle_root = binary_merkle_root(poseidon, leaf, depth, indices, siblings);

    merkle_root
}

#[test]
fn test_get_merkle_root() {
    let address = 0xf39fd6e51aad88f6f4ce6ab8827279cfffb92266;
    let balance = 1000000000000000000;

    // Consider a tree with depth 3, leaves are [poseidon([address, balance]), 0, 0].
    let depth = 2;
    let indices: [u1; 20] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let siblings: [Field; 20] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

    let merkle_root = get_merkle_root(address, balance, depth, indices, siblings);
    let expected = 0x11c1a161d2d84845a176670f3501200b4a0df1048073c0caea6ead2e71b7edaf;

    println(f"merkle_root: {merkle_root}");
    println(f"expected: {expected}");

    assert(merkle_root == expected);
}

#[test]
fn test_get_merkle_root_ts_sdk() {
    let address = 0x1234567890123456789012345678901234567890;
    let balance = 1000;

    let depth = 4;
    let indices: [u1; 20] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
    let siblings: [Field; 20] = [
        3427403932201889290042771220465494458496044723860771244846412090114641612933,
        142173982782527023041821996079117549632814652929360547003021612347642943458,
        5984039385283196232308319035002972848802042601279200765710389913462508845879,
        13230920196039626330844247773673397239708271370016925050843639396057862269188,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
        0,
    ];

    let merkle_root = get_merkle_root(address, balance, depth, indices, siblings);
    let expected = 1459018155640011149834119814743866717977769794914747043488779523926892000355;
    assert(merkle_root == expected);
}
