import { ReactNode } from 'react'
import { BroadcastVoteRequest, BroadcastVoteResponse, VoteStateLite, VotingRound, EncryptedVote } from '@/model/vote.model'
import { Poll, PollRequestResult, PollResult } from '@/model/poll.model'
import { Identity } from '@semaphore-protocol/core'

export type VoteManagementContextType = {
  isLoading: boolean
  user: { address: string } | null
  semaphoreIdentity: Identity | null
  isRegistering: boolean
  isRegisteredForCurrentRound: boolean
  fetchingMembers: boolean
  currentGroupMembers: string[]
  currentSemaphoreGroupId: bigint | null
  votingRound: VotingRound | null
  roundEndDate: Date | null
  pollOptions: Poll[]
  roundState: VoteStateLite | null
  pastPolls: PollResult[]
  txUrl: string | undefined
  pollResult: PollResult | null
  setPollResult: React.Dispatch<React.SetStateAction<PollResult | null>>
  getWebResultByRound: (round_id: number) => Promise<PollRequestResult | undefined>
  setTxUrl: React.Dispatch<React.SetStateAction<string | undefined>>
  setPollOptions: React.Dispatch<React.SetStateAction<Poll[]>>
  initialLoad: () => Promise<void>
  existNewRound: () => Promise<void>
  getPastPolls: () => Promise<void>
  setVotingRound: React.Dispatch<React.SetStateAction<VotingRound | null>>
  setUser: React.Dispatch<React.SetStateAction<{ address: string } | null>>
  encryptVote: (voteId: bigint, publicKey: Uint8Array) => Promise<EncryptedVote | undefined>
  registerIdentityOnContract: () => void
  broadcastVote: (vote: BroadcastVoteRequest) => Promise<BroadcastVoteResponse | undefined>
  getRoundStateLite: (roundCount: number) => Promise<void>
  setPastPolls: React.Dispatch<React.SetStateAction<PollResult[]>>
  getWebResult: () => Promise<PollRequestResult[] | undefined>
}

export type VoteManagementProviderProps = {
  children: ReactNode
}
