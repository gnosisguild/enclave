// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { poseidon2 } from 'poseidon-lite'
import { LeanIMT } from '@zk-kit/lean-imt'
import type { MerkleProof } from './types'
import { MERKLE_TREE_MAX_DEPTH, SIGNATURE_MESSAGE_HASH } from './constants'
import { publicKeyToAddress } from 'viem/utils'
import { hexToBytes, recoverPublicKey } from 'viem'

/**
 * Hash a leaf node for the Merkle tree
 * @param address The voter's address
 * @param balance The voter's balance
 * @returns The hashed leaf as a bigint
 */
export const hashLeaf = (address: string, balance: bigint): bigint => {
  return poseidon2([address.toLowerCase(), balance])
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
 * @param balance The voter's balance
 * @param address The voter's address
 * @param leaves The leaves of the Merkle tree
 */
export const generateMerkleProof = (balance: bigint, address: string, leaves: bigint[] | string[]): MerkleProof => {
  const leaf = hashLeaf(address.toLowerCase(), balance)

  const index = leaves.findIndex((l) => BigInt(l) === leaf)

  if (index === -1) {
    throw new Error('Leaf not found in the tree')
  }

  const tree = generateMerkleTree(leaves.map((l) => BigInt(l)))

  const proof = tree.generateProof(index)

  // Pad siblings with zeros
  const paddedSiblings = [...proof.siblings, ...Array(MERKLE_TREE_MAX_DEPTH - proof.siblings.length).fill(0n)]
  // Pad indices with zeros
  const indices = proof.siblings.map((_, i) => Number((BigInt(proof.index) >> BigInt(i)) & 1n))
  const paddedIndices = [...indices, ...Array(MERKLE_TREE_MAX_DEPTH - indices.length).fill(0)]

  return {
    leaf,
    index,
    proof: {
      ...proof,
      siblings: paddedSiblings,
    },
    // Original length before padding
    length: proof.siblings.length,
    indices: paddedIndices,
  }
}

/**
 * Convert a number to its binary representation
 * @param number The number to convert to binary
 * @returns The binary representation of the number as a string
 */
export const toBinary = (number: bigint): string => {
  if (number < 0) {
    throw new Error('Value cannot be negative')
  }

  return number.toString(2)
}

/**
 * Given a signature, extract the signature components for the Noir signature verification circuit.
 * @param signature The signature to extract the components from.
 * @returns The extracted signature components.
 */
export const extractSignatureComponents = async (
  signature: `0x${string}`,
): Promise<{
  messageHash: Uint8Array
  publicKeyX: Uint8Array
  publicKeyY: Uint8Array
  signature: Uint8Array
}> => {
  const publicKey = await recoverPublicKey({ hash: SIGNATURE_MESSAGE_HASH, signature })
  const publicKeyBytes = hexToBytes(publicKey)
  const publicKeyX = publicKeyBytes.slice(1, 33)
  const publicKeyY = publicKeyBytes.slice(33, 65)

  // Extract r and s from signature (remove v)
  const sigBytes = hexToBytes(signature)
  const r = sigBytes.slice(0, 32) // First 32 bytes
  const s = sigBytes.slice(32, 64) // Next 32 bytes

  const signatureBytes = new Uint8Array(64)
  signatureBytes.set(r, 0)
  signatureBytes.set(s, 32)

  return {
    messageHash: hexToBytes(SIGNATURE_MESSAGE_HASH),
    publicKeyX: publicKeyX,
    publicKeyY: publicKeyY,
    signature: signatureBytes,
  }
}

export const getAddressFromSignature = async (signature: `0x${string}`): Promise<string> => {
  const publicKey = await recoverPublicKey({ hash: SIGNATURE_MESSAGE_HASH, signature })

  return publicKeyToAddress(publicKey)
}

/**
 * Get optimal number of threads for proof generation
 * Leaves at least 1 core free for other operations
 * Works in both Node.js and browser environments
 */
export async function getOptimalThreadCount(): Promise<number> {
  // Node.js environment - use os module if available
  if (typeof process !== 'undefined' && process.versions?.node) {
    try {
      const os = await import('os')
      const cpuCount = typeof os.availableParallelism === 'function' ? os.availableParallelism() : os.cpus().length
      return Math.max(1, cpuCount - 1)
    } catch {
      // Fall through to browser check or fallback
    }
  }

  // Browser environment
  if (typeof navigator !== 'undefined' && navigator.hardwareConcurrency) {
    return Math.max(1, navigator.hardwareConcurrency - 1)
  }

  // Fallback
  return 5
}
