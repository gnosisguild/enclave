// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { handleGenericError } from '@/utils/handle-generic-error'
import {
  BroadcastVoteRequest,
  BroadcastVoteResponse,
  CurrentRound,
  EligibleVoter,
  VoteStateLite,
  VoteStatusRequest,
  VoteStatusResponse,
} from '@/model/vote.model'
import { useApi } from '../generic/useFetchApi'
import { PollRequestResult } from '@/model/poll.model'
import { ROUND_REQUESTERS } from '@/utils/constants'

const ENCLAVE_API = import.meta.env.VITE_ENCLAVE_API

if (!ENCLAVE_API) handleGenericError('useEnclaveServer', { name: 'ENCLAVE_API', message: 'Missing env VITE_ENCLAVE_API' })

const EnclaveEndpoints = {
  GetCurrentRound: `${ENCLAVE_API}/rounds/current`,
  GetRoundStateLite: `${ENCLAVE_API}/state/lite`,
  GetWebResult: `${ENCLAVE_API}/state/result`,
  GetWebAllResult: `${ENCLAVE_API}/state/all`,
  BroadcastVote: `${ENCLAVE_API}/voting/broadcast`,
  GetVoteStatus: `${ENCLAVE_API}/voting/status`,
  GetEligibleVoters: `${ENCLAVE_API}/state/eligible-addresses`,
  GetMerkleLeaves: `${ENCLAVE_API}/state/token-holders`,
} as const

export const useEnclaveServer = () => {
  const { GetCurrentRound, GetWebAllResult, BroadcastVote, GetRoundStateLite, GetWebResult, GetVoteStatus } = EnclaveEndpoints
  const { fetchData, isLoading } = useApi()
  const getCurrentRound = () => fetchData<CurrentRound, { requesters: string[] }>(GetCurrentRound, 'post', { requesters: ROUND_REQUESTERS })
  const getRoundStateLite = (round_id: number) => fetchData<VoteStateLite, { round_id: number }>(GetRoundStateLite, 'post', { round_id })
  const broadcastVote = (vote: BroadcastVoteRequest) => fetchData<BroadcastVoteResponse, BroadcastVoteRequest>(BroadcastVote, 'post', vote)
  const getWebResult = () =>
    fetchData<PollRequestResult[], { requesters: string[] }>(GetWebAllResult, 'post', { requesters: ROUND_REQUESTERS })
  const getWebResultByRound = (round_id: number) => fetchData<PollRequestResult, { round_id: number }>(GetWebResult, 'post', { round_id })
  const getVoteStatus = (request: VoteStatusRequest) => fetchData<VoteStatusResponse, VoteStatusRequest>(GetVoteStatus, 'post', request)
  const getEligibleVoters = (round_id: number) =>
    fetchData<EligibleVoter[], { round_id: number }>(EnclaveEndpoints.GetEligibleVoters, 'post', { round_id })
  const getMerkleLeaves = (round_id: number) =>
    fetchData<string[], { round_id: number }>(EnclaveEndpoints.GetMerkleLeaves, 'post', { round_id })

  return {
    isLoading,
    getWebResultByRound,
    getWebResult,
    getCurrentRound,
    getRoundStateLite,
    broadcastVote,
    getVoteStatus,
    getEligibleVoters,
    getMerkleLeaves,
  }
}
