// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

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

export type VoteResponseStatus = 'success' | 'failed_broadcast'
export interface BroadcastVoteResponse {
  status: VoteResponseStatus
  tx_hash?: string
  message?: string
  is_vote_update?: boolean
}

export interface VoteStatusRequest {
  round_id: number
  address: string
}

export interface VoteStatusResponse {
  round_id: number
  address: string
  has_voted: boolean
  round_status?: string
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

  credit_mode: CreditMode
  credits?: number
}

export enum CreditMode {
  CONSTANT = '0',
  CUSTOM = '1',
}

export type Vote = bigint[]

export interface EligibleVoter {
  address: string
  balance: number
}
