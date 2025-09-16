// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::ticket::WinnerTicket;
use alloy::primitives::{keccak256, Address, B256};

pub fn committee_leaf(seed: u64, address: Address, ticket_id: u64) -> B256 {
    let mut buf = Vec::with_capacity(6 + 8 + 20 + 8);
    buf.extend_from_slice(b"commit");
    buf.extend_from_slice(&seed.to_be_bytes());
    buf.extend_from_slice(address.as_slice());
    buf.extend_from_slice(&ticket_id.to_be_bytes());
    keccak256(&buf)
}

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
            if idx == i {
                proof.push(r);
            } else if idx == i + 1 {
                proof.push(l);
            }
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
    use crate::ticket::{RegisteredNode, Ticket, WinnerTicket};
    use crate::ticket_sortition::ScoreSortition;
    use alloy::primitives::{keccak256, Address, B256};
    use std::collections::HashSet;

    fn ticket_count_for(i: u64) -> u64 {
        let h = keccak256([b"tickets".as_slice(), &i.to_be_bytes()].concat());
        (u128::from_be_bytes(h.0[0..16].try_into().unwrap()) % 7 + 1) as u64
    }

    fn address_for(i: u64) -> Address {
        let h = keccak256([b"addr".as_slice(), &i.to_be_bytes()].concat());
        let mut bytes20 = [0u8; 20];
        bytes20.copy_from_slice(&h.0[12..32]);
        Address::from(bytes20)
    }

    fn build_nodes_with_global_ticket_ids() -> Vec<RegisteredNode> {
        let mut next_ticket_id: u64 = 1;
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

        let nodes = build_nodes_with_global_ticket_ids();
        assert_eq!(nodes.len(), 10);
        assert!(nodes.iter().all(|n| !n.tickets.is_empty()));

        println!("NODES {:#?}", nodes);

        let mut all_ids = std::collections::HashSet::new();
        for n in &nodes {
            for t in &n.tickets {
                assert!(all_ids.insert(t.ticket_id));
            }
        }

        let committee: Vec<WinnerTicket> = ScoreSortition::new(committee_size)
            .get_committee(seed, &nodes)
            .expect("score sortition should succeed");
        assert_eq!(committee.len(), committee_size);

        println!("COMMITTEE {:#?}", committee);

        {
            let mut set = HashSet::new();
            for w in &committee {
                assert!(set.insert(w.address));
            }
        }

        let root = committee_merkle_root(seed, &committee).expect("non-empty committee");
        assert_ne!(root, B256::ZERO);

        let leaves: Vec<B256> = committee
            .iter()
            .map(|w| committee_leaf(seed, w.address, w.ticket_id))
            .collect();

        for (i, w) in committee.iter().enumerate() {
            let leaf = committee_leaf(seed, w.address, w.ticket_id);
            assert_eq!(leaf, leaves[i]);

            let proof = merkle_proof(&leaves, i).expect("proof for valid index");
            let ok = verify_merkle_proof(root, leaf, i, &proof);
            assert!(ok, "failed to verify proof for committee index {}", i);
        }

        if !committee.is_empty() {
            let leaf0 = committee_leaf(seed, committee[0].address, committee[0].ticket_id);
            let proof0 = merkle_proof(&leaves, 0).unwrap();
            let bad_ok = verify_merkle_proof(root, leaf0, 1, &proof0);
            assert!(!bad_ok, "proof verified at wrong index (should fail)");
        }
    }
}
