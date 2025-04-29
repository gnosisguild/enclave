import { handleGenericError } from '@/utils/handle-generic-error'
import {
  BroadcastVoteRequest,
  BroadcastVoteResponse,
  CurrentRound, GroupIdResponse,
  SemaphoreRegistrationRequest,
  SemaphoreRegistrationResponse,
  VoteStateLite,
} from '@/model/vote.model'
import { useApi } from '../generic/useFetchApi'
import { PollRequestResult } from '@/model/poll.model'


const ENCLAVE_API = import.meta.env.VITE_ENCLAVE_API

if (!ENCLAVE_API) handleGenericError('useEnclaveServer', { name: 'ENCLAVE_API', message: 'Missing env VITE_ENCLAVE_API' })

const EnclaveEndpoints = {
  GetCurrentRound: `${ENCLAVE_API}/rounds/current`,
  GetRoundStateLite: `${ENCLAVE_API}/state/lite`,
  GetWebResult: `${ENCLAVE_API}/state/result`,
  GetWebAllResult: `${ENCLAVE_API}/state/all`,
  BroadcastVote: `${ENCLAVE_API}/voting/broadcast`,
  GetGroupId: `${ENCLAVE_API}/rounds/group`,
  SemaphoreRegister: `${ENCLAVE_API}/rounds/register`,
} as const

export const useEnclaveServer = () => {
  const { GetCurrentRound, GetWebAllResult, BroadcastVote, GetRoundStateLite, GetWebResult, SemaphoreRegister , GetGroupId} = EnclaveEndpoints
  const { fetchData, isLoading } = useApi()
  const getCurrentRound = () => fetchData<CurrentRound>(GetCurrentRound)
  const getRoundStateLite = (round_id: number) => fetchData<VoteStateLite, { round_id: number }>(GetRoundStateLite, 'post', { round_id })
  const broadcastVote = (vote: BroadcastVoteRequest) => fetchData<BroadcastVoteResponse, BroadcastVoteRequest>(BroadcastVote, 'post', vote)
  const getWebResult = () => fetchData<PollRequestResult[], void>(GetWebAllResult, 'get')
  const getWebResultByRound = (round_id: number) => fetchData<PollRequestResult, { round_id: number }>(GetWebResult, 'post', { round_id })
  const getGroupId = (round_id: number) => fetchData<GroupIdResponse, { round_id: number }>(GetGroupId, 'post', { round_id })
  const registerWithSemaphore = (registration: SemaphoreRegistrationRequest) =>
      fetchData<SemaphoreRegistrationResponse, SemaphoreRegistrationRequest>(SemaphoreRegister, 'post', registration)

  return {
    isLoading,
    getWebResultByRound,
    getWebResult,
    getCurrentRound,
    getRoundStateLite,
    broadcastVote,
    getGroupId,
    registerWithSemaphore,
  }
}
