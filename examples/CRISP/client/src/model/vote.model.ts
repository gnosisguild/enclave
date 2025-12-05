// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

export interface VotingConfigRequest {
  round_id: number
  chain_id: number
  voting_address: string
  ciphernode_count: number
  voter_count: number
}

export interface VotingRound {
  round_id: number
  pk_bytes: number[]
}

export interface CurrentRound {
  id: number
}

export interface BroadcastVoteRequest {
  round_id: number
  encoded_proof: string
  address: string
}

export type VoteResponseStatus = 'success' | 'user_already_voted' | 'failed_broadcast'
export interface BroadcastVoteResponse {
  status: VoteResponseStatus
  tx_hash?: string
  message?: string
}

export interface VoteStateLite {
  id: number
  chain_id: number
  enclave_address: string

  status: string
  vote_count: number

  start_time: number
  duration: number
  expiration: number
  start_block: number

  committee_public_key: number[]
  emojis: [string, string]
}
