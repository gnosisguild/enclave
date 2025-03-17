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
  enc_vote_bytes: number[] //bytes
  proof_bytes: number[]
  instances: number[][]
  postId: string
}

export interface BroadcastVoteResponse {
  response: string
  tx_hash: string
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

  committee_public_key: number[]
  emojis: [string, string]
}
