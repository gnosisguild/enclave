// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import type { LeanIMTMerkleProof } from '@zk-kit/lean-imt'

/**
 * Type representing the details of a specific round returned by the CRISP server
 */
export type RoundDetailsResponse = {
  id: string
  chain_id: string
  enclave_address: string
  status: string
  vote_count: string
  start_time: string
  duration: string
  expiration: string
  start_block: string
  committee_public_key: string[]
  emojis: [string, string]
  token_address: string
  balance_threshold: string
}

/**
 * Type representing the details of a specific round in a more convenient format
 */
export type RoundDetails = {
  e3Id: bigint
  chainId: bigint
  enclaveAddress: string
  status: string
  voteCount: bigint
  startTime: bigint
  duration: bigint
  expiration: bigint
  startBlock: bigint
  committeePublicKey: string[]
  emojis: [string, string]
  tokenAddress: string
  balanceThreshold: bigint
}

/**
 * Type representing the token details required for participation in a round
 */
export type TokenDetails = {
  tokenAddress: string
  threshold: bigint
  snapshotBlock: bigint
}

/**
 * Type representing a Merkle proof
 */
export type MerkleProof = {
  leaf: bigint
  index: number
  proof: LeanIMTMerkleProof<bigint>
  length: number
  indices: number[]
}

/**
 * Type representing a vote with power for 'yes' and 'no'
 */
export type Vote = {
  /**
   * The voting power for 'yes' votes
   */
  yes: bigint
  /**
   * The voting power for 'no' votes
   */
  no: bigint
}

/**
 * Type representing a vector with coefficients
 */
export type Polynomial = {
  coefficients: string[]
}

/**
 * Type representing cryptographic parameters
 */
export type GrecoCryptographicParams = {
  q_mod_t: string
  qis: string[]
  k0is: string[]
}

/**
 * Type representing Greco bounds
 */
export type GrecoBoundParams = {
  e_bound: string
  u_bound: string
  k1_low_bound: string
  k1_up_bound: string
  p1_bounds: string[]
  p2_bounds: string[]
  pk_bounds: string[]
  r1_low_bounds: string[]
  r1_up_bounds: string[]
  r2_bounds: string[]
}

/**
 * Type representing Greco parameters
 */
export type GrecoParams = {
  crypto: GrecoCryptographicParams
  bounds: GrecoBoundParams
}

/**
 * The inputs required for the CRISP circuit.
 */
export type CircuitInputs = {
  // Ciphertext Addition Section.
  prev_ct0is: Polynomial[]
  prev_ct1is: Polynomial[]
  sum_ct0is: Polynomial[]
  sum_ct1is: Polynomial[]
  sum_r0is: Polynomial[]
  sum_r1is: Polynomial[]
  // Greco Section.
  params: GrecoParams
  pk_commitment: string
  ct0is: Polynomial[]
  ct1is: Polynomial[]
  pk0is: Polynomial[]
  pk1is: Polynomial[]
  r1is: Polynomial[]
  r2is: Polynomial[]
  p1is: Polynomial[]
  p2is: Polynomial[]
  u: Polynomial
  e0: Polynomial
  e0is: Polynomial[]
  e0_quotients: Polynomial[]
  e1: Polynomial
  k1: Polynomial
  // ECDSA Section.
  public_key_x: string[]
  public_key_y: string[]
  signature: string[]
  hashed_message: string[]
  // Merkle Tree Section.
  merkle_root: string
  merkle_proof_length: string
  merkle_proof_indices: string[]
  merkle_proof_siblings: string[]
  // Slot Address Section.
  slot_address: string
  // Balance Section.
  balance: string
  // Whether this is the first vote for this slot.
  is_first_vote: boolean
}

export type ExecuteCircuitResult = {
  witness: Uint8Array
  returnValue: [Polynomial[][], Polynomial[][]]
}

export type ProofInputs = {
  vote: Vote
  publicKey: Uint8Array
  signature: `0x${string}`
  balance: bigint
  slotAddress: string
  previousCiphertext?: Uint8Array
  merkleProof: MerkleProof
  messageHash?: `0x${string}`
}

export type MaskVoteProofInputs = {
  previousCiphertext?: Uint8Array
  merkleLeaves: string[] | bigint[]
  publicKey: Uint8Array
  balance: bigint
  slotAddress: string
}

export type VoteProofInputs = {
  merkleLeaves: string[] | bigint[]
  publicKey: Uint8Array
  balance: bigint
  vote: Vote
  signature: `0x${string}`
  previousCiphertext?: Uint8Array
  messageHash: `0x${string}`
}
