import { ReactNode } from 'react'
import { BroadcastVoteRequest, BroadcastVoteResponse, VoteStateLite, VotingRound } from '@/model/vote.model'
import { Poll, PollRequestResult, PollResult } from '@/model/poll.model'
import { StatusAPIResponse } from '@farcaster/auth-client'
import { Auth } from '@/model/auth.model'

export type VoteManagementContextType = {
  isLoading: boolean
  user: StatusAPIResponse | null
  votingRound: VotingRound | null
  roundEndDate: Date | null
  pollOptions: Poll[]
  roundState: VoteStateLite | null
  pastPolls: PollResult[]
  txUrl: string | undefined
  pollResult: PollResult | null
  setPollResult: React.Dispatch<React.SetStateAction<PollResult | null>>
  getWebResultByRound: (round_id: number) => Promise<PollRequestResult | undefined>
  getToken: (postId: string) => Promise<Auth | undefined>
  setTxUrl: React.Dispatch<React.SetStateAction<string | undefined>>
  setPollOptions: React.Dispatch<React.SetStateAction<Poll[]>>
  initialLoad: () => Promise<void>
  existNewRound: () => Promise<void>
  getPastPolls: () => Promise<void>
  setVotingRound: React.Dispatch<React.SetStateAction<VotingRound | null>>
  setUser: (value: StatusAPIResponse | null) => void
  encryptVote: (voteId: bigint, publicKey: Uint8Array) => Promise<Uint8Array | undefined>
  broadcastVote: (vote: BroadcastVoteRequest) => Promise<BroadcastVoteResponse | undefined>
  getRoundStateLite: (roundCount: number) => Promise<void>
  setPastPolls: React.Dispatch<React.SetStateAction<PollResult[]>>
  getWebResult: () => Promise<PollRequestResult[] | undefined>
  logout: () => void
}

export type VoteManagementProviderProps = {
  children: ReactNode
}
