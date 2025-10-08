// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

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
  tokenAddress: string
  snapshotBlock: string
}

/**
 * Interface representing the details of a specific round in a more convenient format
 */
export interface IRoundDetails {
  e3Id: number
  chainId: number
  enclaveAddress: string
  status: string
  voteCount: number
  startTime: number
  duration: number
  expiration: number
  startBlock: number
  committeePublicKey: string[]
  emojis: [string, string]
  tokenAddress: string
  balanceThreshold: string
  snapshotBlock: number
}

/**
 * Interface representing the token details required for participation in a round
 */
export interface ITokenDetails {
  tokenAddress: string
  threshold: string
}
