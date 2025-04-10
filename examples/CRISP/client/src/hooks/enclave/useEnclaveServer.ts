import { handleGenericError } from '@/utils/handle-generic-error'
import { BroadcastVoteRequest, BroadcastVoteResponse, CurrentRound, VoteStateLite } from '@/model/vote.model'
import { useApi } from '../generic/useFetchApi'
import { PollRequestResult } from '@/model/poll.model'
import { Auth } from '@/model/auth.model'


const ENCLAVE_API = import.meta.env.VITE_ENCLAVE_API

if (!ENCLAVE_API) handleGenericError('useEnclaveServer', { name: 'ENCLAVE_API', message: 'Missing env VITE_ENCLAVE_API' })

const EnclaveEndpoints = {
  GetCurrentRound: `${ENCLAVE_API}/rounds/current`,
  GetRoundStateLite: `${ENCLAVE_API}/state/lite`,
  GetWebResult: `${ENCLAVE_API}/state/result`,
  GetWebAllResult: `${ENCLAVE_API}/state/all`,
  BroadcastVote: `${ENCLAVE_API}/voting/broadcast`,
  Authentication: `${ENCLAVE_API}/auth/login`,
} as const

export const useEnclaveServer = () => {
  const { GetCurrentRound, GetWebAllResult, BroadcastVote, GetRoundStateLite, Authentication, GetWebResult } = EnclaveEndpoints
  const { fetchData, isLoading } = useApi()
  const getCurrentRound = () => fetchData<CurrentRound>(GetCurrentRound)
  const getToken = (postId: string) => fetchData<Auth, { postId: string }>(Authentication, 'post', { postId })
  const getRoundStateLite = (round_id: number) => fetchData<VoteStateLite, { round_id: number }>(GetRoundStateLite, 'post', { round_id })
  const broadcastVote = (vote: BroadcastVoteRequest) => fetchData<BroadcastVoteResponse, BroadcastVoteRequest>(BroadcastVote, 'post', vote)
  const getWebResult = () => fetchData<PollRequestResult[], void>(GetWebAllResult, 'get')
  const getWebResultByRound = (round_id: number) => fetchData<PollRequestResult, { round_id: number }>(GetWebResult, 'post', { round_id })

  return {
    isLoading,
    getWebResultByRound,
    getToken,
    getWebResult,
    getCurrentRound,
    getRoundStateLite,
    broadcastVote,
  }
}
