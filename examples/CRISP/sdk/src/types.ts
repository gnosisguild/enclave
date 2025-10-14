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
}

/**
 * Enum representing the voting modes
 */
export enum VotingMode {
  /**
   *  Governance voting requires to spend all credits on one option 
      they cannot be split
   */
  GOVERNANCE = 'GOVERNANCE',
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
