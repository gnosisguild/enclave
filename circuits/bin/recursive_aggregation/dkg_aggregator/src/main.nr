// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! DKG aggregator (on-chain target): one non-ZK `nodes_fold` proof (folded `H` `node_fold` rows), then
//! C5 with ZK `verify_honk_proof` (base `pk_aggregation` remains ZK in the leaf pipeline).

use bb_proof_verification::{
    UltraHonkProof, UltraHonkVerificationKey, UltraHonkZKProof, verify_honk_proof,
    verify_honk_proof_non_zk,
};
use lib::configs::default::dkg::L_THRESHOLD;
use lib::configs::default::{H, N_PARTIES};
use lib::math::commitments::compute_vk_hash;

/// Must match `node_fold::NODE_FOLD_PUBLIC_LEN`.
pub global NODE_FOLD_PUBLIC_LEN: u32 = 11 + N_PARTIES + (2 * (N_PARTIES + H) * L_THRESHOLD);

/// Must match `nodes_fold::NODES_FOLD_PUBLIC_LEN` (pub params + flattened `H` node-fold rows).
pub global NODES_FOLD_PUBLIC_LEN: u32 = 4 + (H * NODE_FOLD_PUBLIC_LEN);

fn main(
    nodes_fold_vk: UltraHonkVerificationKey,
    nodes_fold_proof: UltraHonkProof,
    nodes_fold_public: [Field; NODES_FOLD_PUBLIC_LEN],
    c5_vk: UltraHonkVerificationKey,
    c5_proof: UltraHonkZKProof,
    c5_public: [Field; H + 1],
    nodes_fold_key_hash: pub Field,
    c5_key_hash: pub Field,
    party_ids: pub [Field; H],
) -> pub (Field, [Field; H], [Field; H], Field) {
    verify_honk_proof_non_zk(
        nodes_fold_vk,
        nodes_fold_proof,
        nodes_fold_public,
        nodes_fold_key_hash,
    );
    verify_honk_proof(c5_vk, c5_proof, c5_public, c5_key_hash);

    // Bind each folded `node_fold` row to the aggregator-supplied honest-set ordering: slot 0 per row
    // is the party id (matches `party_ids[i]`).
    for i in 0..H {
        assert(nodes_fold_public[4 + i * NODE_FOLD_PUBLIC_LEN] == party_ids[i]);
    }

    // Cross-node C2 share grid (see `node_fold` public layout).
    for i in 0..H {
        for j in 0..H {
            assert(
                nodes_fold_public[4 + i * NODE_FOLD_PUBLIC_LEN + 9 + j]
                    == nodes_fold_public[4 + j * NODE_FOLD_PUBLIC_LEN + 7],
            );
        }
    }

    // Cross-party C2 to C4 (first chain).
    for i in 0..H {
        for j in 0..H {
            for l in 0..L_THRESHOLD {
                assert(
                    nodes_fold_public[4 + j * NODE_FOLD_PUBLIC_LEN + 24 + i * L_THRESHOLD + l]
                        == nodes_fold_public[4 + i * NODE_FOLD_PUBLIC_LEN + 12 + j * L_THRESHOLD + l
                            ],
                );
            }
        }
    }

    // Cross-party C2 to C4 (second chain).
    for i in 0..H {
        for j in 0..H {
            for l in 0..L_THRESHOLD {
                assert(
                    nodes_fold_public[4 + j * NODE_FOLD_PUBLIC_LEN + 30 + i * L_THRESHOLD + l]
                        == nodes_fold_public[4 + i * NODE_FOLD_PUBLIC_LEN + 18 + j * L_THRESHOLD + l
                            ],
                );
            }
        }
    }

    // Cross-circuit C5 to `node_fold`: C5 row `i` matches public slot 8 in that node's row.
    for i in 0..H {
        assert(c5_public[i] == nodes_fold_public[4 + i * NODE_FOLD_PUBLIC_LEN + 8]);
    }

    let mut vk_hashes = Vec::new();
    vk_hashes.push(nodes_fold_key_hash);
    vk_hashes.push(c5_key_hash);
    let key_hash = compute_vk_hash(vk_hashes);

    let mut sk_agg_commits: [Field; H] = [0; H];
    let mut esm_agg_commits: [Field; H] = [0; H];
    for i in 0..H {
        sk_agg_commits[i] = nodes_fold_public[4 + i * NODE_FOLD_PUBLIC_LEN + 36];
        esm_agg_commits[i] = nodes_fold_public[4 + i * NODE_FOLD_PUBLIC_LEN + 37];
    }
    let aggregated_pk_commit = c5_public[H];

    (key_hash, sk_agg_commits, esm_agg_commits, aggregated_pk_commit)
}
