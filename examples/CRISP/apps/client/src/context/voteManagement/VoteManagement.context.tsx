import { createGenericContext } from '@/utils/create-generic-context'
import { VoteManagementContextType, VoteManagementProviderProps } from '@/context/voteManagement'
import { useWebAssemblyHook } from '@/hooks/wasm/useWebAssembly'
import { useEffect, useState } from 'react'
import { useAccount } from 'wagmi'
import { VoteStateLite, VotingRound } from '@/model/vote.model'
import { useEnclaveServer } from '@/hooks/enclave/useEnclaveServer'
import { convertPollData, convertTimestampToDate } from '@/utils/methods'
import { Poll, PollResult } from '@/model/poll.model'
import { generatePoll } from '@/utils/generate-random-poll'
import { handleGenericError } from '@/utils/handle-generic-error'

const [useVoteManagementContext, VoteManagementContextProvider] = createGenericContext<VoteManagementContextType>()

const VoteManagementProvider = ({ children }: VoteManagementProviderProps) => {
  /**
   * Wagmi Account State
   **/
  const { address, isConnected } = useAccount()

  /**
   * Voting Management States
   **/
  const [user, setUser] = useState<{ address: string } | null>(null)
  const [roundState, setRoundState] = useState<VoteStateLite | null>(null)
  const [votingRound, setVotingRound] = useState<VotingRound | null>(null)
  const [roundEndDate, setRoundEndDate] = useState<Date | null>(null)
  const [isLoading, setIsLoading] = useState<boolean>(false)
  const [pollOptions, setPollOptions] = useState<Poll[]>([])
  const [pastPolls, setPastPolls] = useState<PollResult[]>([])
  const [txUrl, setTxUrl] = useState<string | undefined>(undefined)
  const [pollResult, setPollResult] = useState<PollResult | null>(null)

  /**
   * Voting Management Methods
   **/
  const { isLoading: wasmLoading, encryptVote } = useWebAssemblyHook()
  const {
    isLoading: enclaveLoading,
    getRoundStateLite: getRoundStateLiteRequest,
    getWebResultByRound,
    getWebResult,
    getCurrentRound,
    broadcastVote,
  } = useEnclaveServer()

  const initialLoad = async () => {
    console.log("Loading wasm");
    const currentRound = await getCurrentRound()
    if (currentRound) {
      await getRoundStateLite(currentRound.id)
    }
  }

  const existNewRound = async () => {
    const currentRound = await getCurrentRound()
    if (currentRound && votingRound && currentRound.id > votingRound.round_id) {
      await getRoundStateLite(currentRound.id)
    }
  }

  const logout = () => {
    setUser(null)
  }

  const getRoundStateLite = async (roundCount: number) => {
    const roundState = await getRoundStateLiteRequest(roundCount)

    if (roundState?.committee_public_key.length === 1 && roundState.committee_public_key[0] === 0) {
      handleGenericError('getRoundStateLite', {
        message: 'Enclave server failed generating the necessary pk bytes',
        name: 'getRoundStateLite',
      })
    }
    if (roundState) {
      setRoundState(roundState)
      setVotingRound({ round_id: roundState.id, pk_bytes: roundState.committee_public_key })
      setPollOptions(generatePoll({ round_id: roundState.id, emojis: roundState.emojis }))
      setRoundEndDate(convertTimestampToDate(roundState.start_time, roundState.duration))
    }
  }

  const getPastPolls = async () => {
    try {
      const result = await getWebResult()
      if (result) {
        const convertedPolls = convertPollData(result)
        setPastPolls(convertedPolls)
      }
    } catch (error) {
      handleGenericError('getPastPolls', error as Error)
    } finally {
      setIsLoading(false)
    }
  }

  useEffect(() => {
    if ([wasmLoading, enclaveLoading].includes(true)) {
      return setIsLoading(true)
    }
    setIsLoading(false)
  }, [wasmLoading, enclaveLoading])

  // Update user state when wallet connection changes
  useEffect(() => {
    if (isConnected && address) {
      setUser({ address })
    } else {
      setUser(null)
    }
  }, [isConnected, address])

  return (
    <VoteManagementContextProvider
      value={{
        isLoading,
        user,
        votingRound,
        roundEndDate,
        pollOptions,
        roundState,
        pastPolls,
        txUrl,
        pollResult,
        setPollResult,
        getWebResultByRound,
        setTxUrl,
        existNewRound,
        getWebResult,
        setPastPolls,
        getPastPolls,
        getRoundStateLite,
        setPollOptions,
        initialLoad,
        broadcastVote,
        setVotingRound,
        setUser,
        encryptVote,
        logout,
      }}
    >
      {children}
    </VoteManagementContextProvider>
  )
}

export { useVoteManagementContext, VoteManagementProvider }
