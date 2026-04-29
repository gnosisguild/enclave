// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! DKG aggregator: non-ZK `nodes_fold` and non-ZK `pk_aggregation` (both `noir-recursive-no-zk`).
//! The final `bb prove -t evm` proof of this circuit is what the contract verifies on-chain.

use bb_proof_verification::{UltraHonkProof, UltraHonkVerificationKey, verify_honk_proof_non_zk};
use lib::configs::default::dkg::L_THRESHOLD;
use lib::configs::default::{H, N_PARTIES};
use lib::math::commitments::compute_vk_hash;

/// Must match `node_fold::NODE_FOLD_PUBLIC_LEN`.
pub global NODE_FOLD_PUBLIC_LEN: u32 = 11 + N_PARTIES + (2 * (N_PARTIES + H) * L_THRESHOLD);

/// Must match `nodes_fold::NODES_FOLD_PUBLIC_LEN` (pub params + flattened `H` node-fold rows).
pub global NODES_FOLD_PUBLIC_LEN: u32 = 4 + (H * NODE_FOLD_PUBLIC_LEN);

/// Flattened `node_fold` return tuple length (key hash + pks + share grids + agg commits).
pub global NODE_FOLD_RETURN_PUBLIC_LEN: u32 =
    5 + N_PARTIES + (2 * N_PARTIES * L_THRESHOLD) + (2 * H * L_THRESHOLD);

/// Leading `node_fold` public inputs (party id + five key hashes) before the return tuple.
pub global NODE_FOLD_PARAM_PUBLIC_LEN: u32 = NODE_FOLD_PUBLIC_LEN - NODE_FOLD_RETURN_PUBLIC_LEN;

/// Leading `nodes_fold` public inputs before the `H` full `node_fold` rows.
pub global NODES_FOLD_PARAM_PUBLIC_LEN: u32 = NODES_FOLD_PUBLIC_LEN - (H * NODE_FOLD_PUBLIC_LEN);

/// Indices within one `node_fold` public row (length `NODE_FOLD_PUBLIC_LEN`).
pub global NF_PARTY_ID_IDX: u32 = 0;
pub global NF_C0_PK_IDX: u32 = NODE_FOLD_PARAM_PUBLIC_LEN + 1;
pub global NF_C1_PK_IDX: u32 = NODE_FOLD_PARAM_PUBLIC_LEN + 2;
pub global NF_C3_PK_BASE_IDX: u32 = NODE_FOLD_PARAM_PUBLIC_LEN + 3;
pub global NF_C2A_BASE_IDX: u32 = NODE_FOLD_PARAM_PUBLIC_LEN + 3 + N_PARTIES;
pub global NF_C2B_BASE_IDX: u32 = NF_C2A_BASE_IDX + (N_PARTIES * L_THRESHOLD);
pub global NF_C4A_BASE_IDX: u32 = NF_C2B_BASE_IDX + (N_PARTIES * L_THRESHOLD);
pub global NF_C4B_BASE_IDX: u32 = NF_C4A_BASE_IDX + (H * L_THRESHOLD);
pub global NF_SK_AGG_IDX: u32 = NF_C4B_BASE_IDX + (H * L_THRESHOLD);
pub global NF_ESM_AGG_IDX: u32 = NF_SK_AGG_IDX + 1;

fn main(
    nodes_fold_vk: UltraHonkVerificationKey,
    nodes_fold_proof: UltraHonkProof,
    nodes_fold_public: [Field; NODES_FOLD_PUBLIC_LEN],
    c5_vk: UltraHonkVerificationKey,
    c5_proof: UltraHonkProof,
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
    verify_honk_proof_non_zk(c5_vk, c5_proof, c5_public, c5_key_hash);

    // Bind each folded `node_fold` row to the aggregator-supplied honest-set ordering: slot 0 per row
    // is the party id (matches `party_ids[i]`).
    for i in 0..H {
        assert(
            nodes_fold_public[NODES_FOLD_PARAM_PUBLIC_LEN
                + i * NODE_FOLD_PUBLIC_LEN
                + NF_PARTY_ID_IDX]
                == party_ids[i],
        );
    }

    // Cross-node C0 to C3: each row's c3_pk[j] (recipient pk used in party i's C3 encryption for
    // recipient j) must equal party j's own c0_pk.
    for i in 0..H {
        for j in 0..H {
            assert(
                nodes_fold_public[NODES_FOLD_PARAM_PUBLIC_LEN
                    + i * NODE_FOLD_PUBLIC_LEN
                    + NF_C3_PK_BASE_IDX
                    + j]
                    == nodes_fold_public[NODES_FOLD_PARAM_PUBLIC_LEN
                        + j * NODE_FOLD_PUBLIC_LEN
                        + NF_C0_PK_IDX],
            );
        }
    }

    // Cross-party C2 to C4 (first chain).
    for i in 0..H {
        for j in 0..H {
            for l in 0..L_THRESHOLD {
                assert(
                    nodes_fold_public[NODES_FOLD_PARAM_PUBLIC_LEN
                        + j * NODE_FOLD_PUBLIC_LEN
                        + NF_C4A_BASE_IDX
                        + i * L_THRESHOLD
                        + l]
                        == nodes_fold_public[NODES_FOLD_PARAM_PUBLIC_LEN
                            + i * NODE_FOLD_PUBLIC_LEN
                            + NF_C2A_BASE_IDX
                            + j * L_THRESHOLD
                            + l],
                );
            }
        }
    }

    // Cross-party C2 to C4 (second chain).
    for i in 0..H {
        for j in 0..H {
            for l in 0..L_THRESHOLD {
                assert(
                    nodes_fold_public[NODES_FOLD_PARAM_PUBLIC_LEN
                        + j * NODE_FOLD_PUBLIC_LEN
                        + NF_C4B_BASE_IDX
                        + i * L_THRESHOLD
                        + l]
                        == nodes_fold_public[NODES_FOLD_PARAM_PUBLIC_LEN
                            + i * NODE_FOLD_PUBLIC_LEN
                            + NF_C2B_BASE_IDX
                            + j * L_THRESHOLD
                            + l],
                );
            }
        }
    }

    // Cross-circuit C5 to `node_fold`: C5 row `i` matches that node's `c1_pk` public field.
    for i in 0..H {
        assert(
            c5_public[i]
                == nodes_fold_public[NODES_FOLD_PARAM_PUBLIC_LEN
                    + i * NODE_FOLD_PUBLIC_LEN
                    + NF_C1_PK_IDX],
        );
    }

    let mut vk_hashes = Vec::new();
    vk_hashes.push(nodes_fold_key_hash);
    vk_hashes.push(c5_key_hash);
    let key_hash = compute_vk_hash(vk_hashes);

    let mut sk_agg_commits: [Field; H] = [0; H];
    let mut esm_agg_commits: [Field; H] = [0; H];
    for i in 0..H {
        sk_agg_commits[i] = nodes_fold_public[NODES_FOLD_PARAM_PUBLIC_LEN
            + i * NODE_FOLD_PUBLIC_LEN
            + NF_SK_AGG_IDX];
        esm_agg_commits[i] = nodes_fold_public[NODES_FOLD_PARAM_PUBLIC_LEN
            + i * NODE_FOLD_PUBLIC_LEN
            + NF_ESM_AGG_IDX];
    }
    let aggregated_pk_commit = c5_public[H];

    (key_hash, sk_agg_commits, esm_agg_commits, aggregated_pk_commit)
}
