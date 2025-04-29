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

export type NumericString = string;
export type PackedGroth16Proof = [NumericString, NumericString, NumericString, NumericString, NumericString, NumericString, NumericString, NumericString];

// Semaphore proof structure as required by the verifier
export interface SemaphoreProof {
  merkleTreeDepth: number;
  merkleTreeRoot: NumericString;
  message: NumericString;
  nullifier: NumericString;
  scope: NumericString;
  points: PackedGroth16Proof;
}
export interface BroadcastVoteRequest {
  round_id: number
  enc_vote_bytes: number[] //bytes
  address: string
  proof_sem: SemaphoreProof
}

export type VoteResponseStatus = 'success' | 'user_already_voted' | 'failed_broadcast';
export interface BroadcastVoteResponse {
  status: VoteResponseStatus;
  tx_hash?: string;
  message?: string;
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


export interface SemaphoreRegistrationRequest {
  round_id: number
  identity_commitment: string
  group_id: number
}

export interface SemaphoreRegistrationResponse {
  response: string
}

export interface GroupIdResponse {
  group_id: string;
  exists: boolean;
}