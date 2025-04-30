import { createGenericContext } from '@/utils/create-generic-context'
import { VoteManagementContextType, VoteManagementProviderProps } from '@/context/voteManagement'
import { useWebAssemblyHook } from '@/hooks/wasm/useWebAssembly'
import { useEffect, useState } from 'react'
import { useAccount } from 'wagmi'
import { Identity } from '@semaphore-protocol/core/identity'
import useLocalStorage from '@/hooks/generic/useLocalStorage'
import { VoteStateLite, VotingRound } from '@/model/vote.model'
import { useEnclaveServer } from '@/hooks/enclave/useEnclaveServer'
import { convertPollData, convertTimestampToDate } from '@/utils/methods'
import { Poll, PollResult } from '@/model/poll.model'
import { generatePoll } from '@/utils/generate-random-poll'
import { handleGenericError } from '@/utils/handle-generic-error'
import { useSemaphoreGroupManagement } from '@/hooks/semaphore/useSemaphoreGroupManagement'

const [useVoteManagementContext, VoteManagementContextProvider] = createGenericContext<VoteManagementContextType>()

const VoteManagementProvider = ({ children }: VoteManagementProviderProps) => {
  /**
   * Wagmi Account State
   **/
  const { address, isConnected } = useAccount()

  /**
   * Voting Management States
   **/
  const [identityPrivateKey, setIdentityPrivateKey] = useLocalStorage<string | undefined>('semaphoreIdentity', undefined)
  const [semaphoreIdentity, setSemaphoreIdentity] = useState<Identity | null>(null)
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

  const {
    groupId: currentSemaphoreGroupId,
    groupMembers: currentGroupMembers,
    isFetchingMembers: fetchingMembers,
    isRegistering,
    isCommitted: isRegisteredForCurrentRound,
    registerIdentity: registerIdentityOnContract
  } = useSemaphoreGroupManagement(roundState?.id, semaphoreIdentity);

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

  const getRoundStateLite = async (roundCount: number) => {
    const fetchedRoundState = await getRoundStateLiteRequest(roundCount);

    if (fetchedRoundState?.committee_public_key.length === 1 && fetchedRoundState.committee_public_key[0] === 0) {
      handleGenericError('getRoundStateLite', {
        message: 'Enclave server failed generating the necessary pk bytes',
        name: 'getRoundStateLite',
      });
    }
    if (fetchedRoundState) {
      setRoundState(fetchedRoundState);
      setVotingRound({ round_id: fetchedRoundState.id, pk_bytes: fetchedRoundState.committee_public_key });
      setPollOptions(generatePoll({ round_id: fetchedRoundState.id, emojis: fetchedRoundState.emojis }));
      setRoundEndDate(convertTimestampToDate(fetchedRoundState.start_time, fetchedRoundState.duration));
    }
  };

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

  useEffect(() => {
    if (semaphoreIdentity) {
      return;
    }
    let identity: Identity | null = null;
    if (identityPrivateKey) {
      try {
        identity = Identity.import(identityPrivateKey);
        console.log('Semaphore identity loaded from storage.');
      } catch (error) {
        console.error('Failed to import Semaphore identity from storage. Will generate a new one.', error);
      }
    } else {
      identity = new Identity();
      setIdentityPrivateKey(identity.export());
      console.log('New Semaphore identity generated and saved.');
    }
    setSemaphoreIdentity(identity);
  }, [identityPrivateKey, semaphoreIdentity]);

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
        semaphoreIdentity,
        votingRound,
        isRegistering,
        fetchingMembers,
        currentSemaphoreGroupId,
        currentGroupMembers,
        isRegisteredForCurrentRound,
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
        registerIdentityOnContract,
        setVotingRound,
        setUser,
        encryptVote,
      }}
    >
      {children}
    </VoteManagementContextProvider>
  )
}

export { useVoteManagementContext, VoteManagementProvider }
