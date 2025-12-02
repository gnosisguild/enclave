// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import type { LeanIMTMerkleProof } from '@zk-kit/lean-imt'

/**
 * Interface representing the details of a specific round returned by the CRISP server
 */
export interface IRoundDetailsResponse {
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
 * Interface representing the details of a specific round in a more convenient format
 */
export interface IRoundDetails {
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
 * Interface representing the token details required for participation in a round
 */
export interface ITokenDetails {
  tokenAddress: string
  threshold: bigint
  snapshotBlock: bigint
}

/**
 * Interface representing a Merkle proof
 */
export interface IMerkleProof {
  leaf: bigint
  index: number
  proof: LeanIMTMerkleProof<bigint>
  length: number
  indices: number[]
}

/**
 * Interface representing a vote with power for 'yes' and 'no'
 */
export interface IVote {
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
 * Interface representing a vector with coefficients
 */
export interface Polynomial {
  coefficients: string[]
}

/**
 * Interface representing cryptographic parameters
 */
export interface GrecoCryptographicParams {
  q_mod_t: string
  qis: string[]
  k0is: string[]
}

/**
 * Interface representing Greco bounds
 */
export interface GrecoBoundParams {
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
 * Interface representing Greco parameters
 */
export interface GrecoParams {
  crypto: GrecoCryptographicParams
  bounds: GrecoBoundParams
}

/**
 * The inputs required for the CRISP circuit.
 */
export interface CircuitInputs {
  // Ciphertext Addition Section.
  prev_ct0is: Polynomial[]
  prev_ct1is: Polynomial[]
  sum_ct0is: Polynomial[]
  sum_ct1is: Polynomial[]
  sum_r0is: Polynomial[]
  sum_r1is: Polynomial[]
  // Greco Section.
  params: GrecoParams
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

export interface ProofInputs {
  vote: IVote
  publicKey: Uint8Array
  signature: `0x${string}`
  balance: bigint
  slotAddress: string
  previousCiphertext?: Uint8Array
  merkleProof: IMerkleProof
}

export interface MaskVoteProofInputs {
  previousCiphertext?: Uint8Array
  merkleLeaves: string[] | bigint[]
  publicKey: Uint8Array
  balance: bigint
  slotAddress: string
}

export interface VoteProofInputs {
  merkleLeaves: string[] | bigint[]
  publicKey: Uint8Array
  balance: bigint
  vote: IVote
  signature: `0x${string}`
}
