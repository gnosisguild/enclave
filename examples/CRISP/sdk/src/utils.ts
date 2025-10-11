// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { poseidon2 } from 'poseidon-lite'
import { LeanIMT } from '@zk-kit/lean-imt'

import type { IMerkleProof } from './types'

/**
 * Hash a leaf node for the Merkle tree
 * @param address The voter's address
 * @param balance The voter's balance
 * @returns The hashed leaf as a bigint
 */
export const hashLeaf = (address: string, balance: string): bigint => {
  return poseidon2([address, balance])
}

/**
 * Generate a new LeanIMT with the leaves provided
 * @param leaves The leaves of the Merkle tree
 * @returns the generated Merkle tree
 */
export const generateMerkleTree = (leaves: bigint[]): LeanIMT => {
  return new LeanIMT((a, b) => poseidon2([a, b]), leaves)
}

/**
 * Generate a Merkle proof for a given address to prove inclusion in the voters' list
 * @param threshold The minimum balance required to be eligible
 * @param balance The voter's balance
 * @param address The voter's address
 * @param leaves The leaves of the Merkle tree
 */
export const generateMerkleProof = (threshold: number, balance: number, address: string, leaves: bigint[]): IMerkleProof => {
  if (balance < threshold) {
    throw new Error('Balance is below the threshold')
  }

  const leaf = hashLeaf(address, balance.toString())

  const index = leaves.findIndex((l) => l === leaf)

  if (index === -1) {
    throw new Error('Leaf not found in the tree')
  }

  const tree = generateMerkleTree(leaves)

  const proof = tree.generateProof(index)

  return {
    leaf,
    index,
    proof,
  }
}
