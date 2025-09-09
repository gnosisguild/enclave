// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::ticket::WinnerTicket;
use alloy::primitives::{keccak256, Address, B256};

/// Compute the deterministic Merkle leaf for a committee member.
///
/// Leaf definition:
/// `keccak256("commit" || seed_be64 || address(20B) || ticket_id_be64)`
///
/// # Parameters
/// - `seed`: Round seed shared by all participants for this round.
/// - `address`: Node’s address (20 bytes).
/// - `ticket_id`: Globally-unique ticket identifier.
///
/// # Returns
/// A 32-byte Keccak hash (`B256`) representing the leaf value.
pub fn committee_leaf(seed: u64, address: Address, ticket_id: u64) -> B256 {
    let mut buf = Vec::with_capacity(6 + 8 + 20 + 8);
    buf.extend_from_slice(b"commit"); // domain tag
    buf.extend_from_slice(&seed.to_be_bytes());
    buf.extend_from_slice(address.as_slice());
    buf.extend_from_slice(&ticket_id.to_be_bytes());
    keccak256(&buf)
}

/// Build a binary Keccak Merkle root from leaves.
///
/// Internal nodes are computed as `keccak256(left || right)`.
/// When a level has an odd number of nodes, the last node is duplicated
/// (i.e., `parent = H(last || last)`).
///
/// # Parameters
/// - `leaves`: Bottom-level leaf hashes in the intended, deterministic order.
///
/// # Returns
/// - `Some(root)` when `leaves` is non-empty,
/// - `None` when `leaves` is empty.
pub fn merkle_root(mut leaves: Vec<B256>) -> Option<B256> {
    if leaves.is_empty() {
        return None;
    }
    if leaves.len() == 1 {
        return Some(leaves[0]);
    }

    while leaves.len() > 1 {
        let mut next_level: Vec<B256> = Vec::with_capacity((leaves.len() + 1) / 2);
        let mut i = 0;
        while i < leaves.len() {
            let l = leaves[i];
            let r = if i + 1 < leaves.len() {
                leaves[i + 1]
            } else {
                l
            };
            let mut buf = [0u8; 64];
            buf[..32].copy_from_slice(l.as_slice());
            buf[32..].copy_from_slice(r.as_slice());
            next_level.push(keccak256(buf));
            i += 2;
        }
        leaves = next_level;
    }
    Some(leaves[0])
}

/// Convenience helper: compute the committee Merkle root from `WinnerTicket`s.
///
/// This maps each `WinnerTicket` to a leaf via [`committee_leaf`], preserving
/// the slice order, then folds them with [`merkle_root`].
///
/// # Parameters
/// - `seed`: Round seed.
/// - `committee`: Ordered slice of winners (length `N`).
///
/// # Returns
/// - `Some(root)` if `committee` is non-empty,
/// - `None` otherwise.
pub fn committee_merkle_root(seed: u64, committee: &[WinnerTicket]) -> Option<B256> {
    if committee.is_empty() {
        return None;
    }
    let leaves: Vec<B256> = committee
        .iter()
        .map(|w| committee_leaf(seed, w.address, w.ticket_id))
        .collect();
    merkle_root(leaves)
}

/// Construct a Merkle inclusion proof (sibling path) for a leaf at `index`.
///
/// The proof contains the sibling hash from each level needed to recompute
/// the root from the target leaf.
///
/// # Parameters
/// - `leaves`: All bottom-level leaves in the same order used to compute the root.
/// - `index`: Zero-based index of the leaf to prove.
///
/// # Returns
/// - `Some(Vec<B256>)` with the sibling hashes when valid,
/// - `None` if `leaves` is empty or `index` is out of bounds.
pub fn merkle_proof(leaves: &[B256], index: usize) -> Option<Vec<B256>> {
    if leaves.is_empty() || index >= leaves.len() {
        return None;
    }
    if leaves.len() == 1 {
        return Some(Vec::new());
    }

    let mut level: Vec<B256> = leaves.to_vec();
    let mut idx = index;
    let mut proof = Vec::new();

    while level.len() > 1 {
        let mut next = Vec::with_capacity((level.len() + 1) / 2);
        for i in (0..level.len()).step_by(2) {
            let l = level[i];
            let r = if i + 1 < level.len() { level[i + 1] } else { l };
            // record sibling if this pair contains the target index
            if idx == i {
                proof.push(r);
            } else if idx == i + 1 {
                proof.push(l);
            }
            // compute parent
            let mut buf = [0u8; 64];
            buf[..32].copy_from_slice(l.as_slice());
            buf[32..].copy_from_slice(r.as_slice());
            next.push(keccak256(buf));
        }
        idx /= 2;
        level = next;
    }

    Some(proof)
}

/// Verify a Merkle inclusion proof for `leaf` at `index` against `root`.
///
/// Rebuilds the hash path by combining `leaf` with each sibling hash from
/// the proof: `acc = keccak256(left || right)`, where `(left, right)` depend
/// on whether the current index is even or odd. The proof is valid iff the
/// final accumulator equals `root`.
///
/// # Parameters
/// - `root`: Expected Merkle root.
/// - `leaf`: Leaf value being proven.
/// - `index`: Zero-based index of the leaf at the bottom level.
/// - `proof`: Sibling path (from bottom to top).
///
/// # Returns
/// `true` if the recomputed value equals `root`, `false` otherwise.
pub fn verify_merkle_proof(root: B256, leaf: B256, mut index: usize, proof: &[B256]) -> bool {
    let mut acc = leaf;
    for sib in proof {
        let (l, r) = if index % 2 == 0 {
            (acc, *sib)
        } else {
            (*sib, acc)
        };
        let mut buf = [0u8; 64];
        buf[..32].copy_from_slice(l.as_slice());
        buf[32..].copy_from_slice(r.as_slice());
        acc = keccak256(buf);
        index /= 2;
    }
    acc == root
}

#[cfg(test)]
mod tests {
    use crate::merkle_committee::{
        committee_leaf, committee_merkle_root, merkle_proof, verify_merkle_proof,
    };
    use crate::ticket::{best_ticket_for_node, RegisteredNode, Ticket, WinnerTicket};
    use crate::ticket_sortition::ScoreSortition;
    use alloy::primitives::{keccak256, Address, B256};
    use std::collections::HashSet;

    /// Deterministic pseudo-random ticket count in [1..=7] per node.
    fn ticket_count_for(i: u64) -> u64 {
        let h = keccak256([b"tickets".as_slice(), &i.to_be_bytes()].concat());
        (u128::from_be_bytes(h.0[0..16].try_into().unwrap()) % 7 + 1) as u64
    }

    /// Deterministic pseudo-random address for node i.
    fn address_for(i: u64) -> Address {
        let h = keccak256([b"addr".as_slice(), &i.to_be_bytes()].concat());
        let mut bytes20 = [0u8; 20];
        bytes20.copy_from_slice(&h.0[12..32]);
        Address::from(bytes20)
    }

    /// Build 10 nodes with **globally unique** ticket IDs.
    fn build_nodes_with_global_ticket_ids() -> Vec<RegisteredNode> {
        let mut next_ticket_id: u64 = 1; // global counter

        (0u64..10)
            .map(|i| {
                let address = address_for(i);
                let t = ticket_count_for(i);
                let mut tickets = Vec::with_capacity(t as usize);
                for _ in 0..t {
                    tickets.push(Ticket {
                        ticket_id: next_ticket_id,
                    });
                    next_ticket_id += 1;
                }
                RegisteredNode { address, tickets }
            })
            .collect()
    }

    #[test]
    fn end_to_end_committee_merkle_global_ticket_ids() {
        let seed: u64 = 0xA1B2_C3D4_E5F6_7788;
        let committee_size: usize = 3;

        // Build nodes and ensure they all have tickets.
        let nodes = build_nodes_with_global_ticket_ids();
        assert_eq!(nodes.len(), 10);
        assert!(nodes.iter().all(|n| !n.tickets.is_empty()));

        println!("NODES {:#?}", nodes);

        // Sanity: all ticket IDs are globally unique.
        let mut all_ids = std::collections::HashSet::new();
        for n in &nodes {
            for t in &n.tickets {
                assert!(
                    all_ids.insert(t.ticket_id),
                    "duplicate global ticket_id {}",
                    t.ticket_id
                );
            }
        }

        // Compute per-node winners.
        let mut winners: Vec<WinnerTicket> = Vec::with_capacity(nodes.len());
        for n in &nodes {
            let w = best_ticket_for_node(seed, n)
                .expect("best_ticket_for_node should succeed when node has tickets");
            assert!(n.tickets.iter().any(|t| t.ticket_id == w.ticket_id));
            assert_eq!(w.address, n.address);
            winners.push(w);
        }
        assert_eq!(winners.len(), 10);

        println!("WINNERS {:#?}", winners);

        // Select committee of size = 3 using (score asc, ticket_id asc).
        let committee = ScoreSortition::new(committee_size)
            .get_committee(&winners)
            .expect("score sortition should succeed");
        assert_eq!(committee.len(), committee_size);

        println!("COMMITTEE {:#?}", committee);

        // Ensure addresses are unique within the committee.
        {
            let mut set = HashSet::new();
            for w in &committee {
                assert!(set.insert(w.address), "duplicate address in committee");
            }
        }

        // Build Merkle root over the ordered committee.
        let root = committee_merkle_root(seed, &committee).expect("non-empty committee");
        assert_ne!(root, B256::ZERO);

        // Generate and verify Merkle proofs for each committee member.
        let leaves: Vec<B256> = committee
            .iter()
            .map(|w| committee_leaf(seed, w.address, w.ticket_id))
            .collect();

        for (i, w) in committee.iter().enumerate() {
            let leaf = committee_leaf(seed, w.address, w.ticket_id);
            assert_eq!(leaf, leaves[i], "leaf mismatch at index {}", i);

            let proof = merkle_proof(&leaves, i).expect("proof for valid index");
            let ok = verify_merkle_proof(root, leaf, i, &proof);
            assert!(ok, "failed to verify proof for committee index {}", i);
        }

        // Negative check: proof must not verify at the wrong index.
        if !committee.is_empty() {
            let leaf0 = committee_leaf(seed, committee[0].address, committee[0].ticket_id);
            let proof0 = merkle_proof(&leaves, 0).unwrap();
            let bad_ok = verify_merkle_proof(root, leaf0, 1, &proof0);
            assert!(!bad_ok, "proof verified at wrong index (should fail)");
        }
    }
}
