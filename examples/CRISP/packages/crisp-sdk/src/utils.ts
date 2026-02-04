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
import { ZKInputsGenerator } from '@crisp-e3/zk-inputs'

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
  messageHash: `0x${string}` = SIGNATURE_MESSAGE_HASH,
): Promise<{
  messageHash: Uint8Array
  publicKeyX: Uint8Array
  publicKeyY: Uint8Array
  signature: Uint8Array
}> => {
  const publicKey = await recoverPublicKey({ hash: messageHash, signature })
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
    messageHash: hexToBytes(messageHash),
    publicKeyX: publicKeyX,
    publicKeyY: publicKeyY,
    signature: signatureBytes,
  }
}

export const getAddressFromSignature = async (signature: `0x${string}`, messageHash?: `0x${string}`): Promise<string> => {
  const publicKey = await recoverPublicKey({ hash: messageHash || SIGNATURE_MESSAGE_HASH, signature })

  return publicKeyToAddress(publicKey)
}

/**
 * Get optimal number of threads for proof generation.
 * Leaves at least 1 core free for other operations.
 * Works in both Node.js and browser environments.
 */
export async function getOptimalThreadCount(): Promise<number> {
  // Browser environment - check first to avoid Node.js imports in browser builds
  if (typeof navigator !== 'undefined' && navigator.hardwareConcurrency) {
    return Math.max(1, navigator.hardwareConcurrency - 1)
  }

  // Node.js environment - use os module if available
  // Check for Node.js without directly accessing process to avoid polyfill detection
  if (typeof window === 'undefined' && typeof globalThis !== 'undefined' && typeof globalThis.process !== 'undefined') {
    try {
      const os = await import('os')

      const cpuCount = typeof os.availableParallelism === 'function' ? os.availableParallelism() : os.cpus().length

      return Math.max(1, cpuCount - 1)
    } catch {
      // Fall through to fallback
    }
  }

  // Fallback
  return 5
}

/**
 * Get the maximum vote value for a given number of choices.
 * @param numChoices Number of choices.
 * @returns Maximum value per choice.
 */
export const getMaxVoteValue = (numChoices: number): bigint => {
  const bfvParams = ZKInputsGenerator.withDefaults().getBFVParams()
  const segmentSize = Math.floor(bfvParams.degree / numChoices)
  return (1n << BigInt(segmentSize)) - 1n
}

/**
 * Get a zero vote with the given number of choices.
 * @param numChoices Number of choices.
 * @returns A zero vote with the given number of choices.
 */
export const getZeroVote = (numChoices: number): bigint[] => {
  return Array(numChoices).fill(0n)
}
