import { createGenericContext } from '@/utils/create-generic-context'
import { VoteManagementContextType, VoteManagementProviderProps } from '@/context/voteManagement'
import { useWebAssemblyHook } from '@/hooks/wasm/useWebAssembly'
import { useEffect, useState } from 'react'
import { useAccount, useWriteContract, useWaitForTransactionReceipt } from 'wagmi'
import { Identity } from '@semaphore-protocol/core/identity'
import useLocalStorage from '@/hooks/generic/useLocalStorage'
import { VoteStateLite, VotingRound } from '@/model/vote.model'
import { useEnclaveServer } from '@/hooks/enclave/useEnclaveServer'
import { convertPollData, convertTimestampToDate } from '@/utils/methods'
import { Poll, PollResult } from '@/model/poll.model'
import { generatePoll } from '@/utils/generate-random-poll'
import { handleGenericError } from '@/utils/handle-generic-error'
import { E3_PROGRAM_ADDRESS, E3_PROGRAM_ABI } from '@/config/contracts'
import { useNotificationAlertContext } from '@/context/NotificationAlert/NotificationAlert.context.tsx'

const [useVoteManagementContext, VoteManagementContextProvider] = createGenericContext<VoteManagementContextType>()

const VoteManagementProvider = ({ children }: VoteManagementProviderProps) => {
  /**
   * Wagmi Account State
   **/
  const { address, isConnected } = useAccount()
  const { data: hash, error: writeError, isPending: isWritePending, writeContract } = useWriteContract();

  /**
   * Notification Hook
   **/
  const { showToast } = useNotificationAlertContext()

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
  const [isRegistering, setIsRegistering] = useState<boolean>(false);
  const [isRegisteredForCurrentRound, setIsRegisteredForCurrentRound] = useState<boolean>(false); // Frontend tracking

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
      setIsRegisteredForCurrentRound(false); // Reset registration status for new round
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

  // Function to register identity on the contract
  const registerIdentityOnContract = async () => {
    if (!roundState || !semaphoreIdentity || !isConnected) {
      console.error('Cannot register: Missing round state, identity, or wallet connection.');
      showToast({ type: 'danger', message: 'Cannot register identity. Ensure wallet is connected and round is active.' });
      return;
    }

    const identityCommitment = semaphoreIdentity.commitment;
    console.log(`Registering commitment: ${identityCommitment} for round: ${roundState.id}`);

    writeContract({
      address: E3_PROGRAM_ADDRESS,
      abi: E3_PROGRAM_ABI,
      functionName: 'registerMember',
      args: [BigInt(roundState.id), identityCommitment],
    });
  };

  // Monitor registration transaction
  const { isLoading: isConfirming, isSuccess: isConfirmed, error: confirmationError } = useWaitForTransactionReceipt({ hash });

  useEffect(() => {
    setIsRegistering(isWritePending || isConfirming);
  }, [isWritePending, isConfirming]);

  useEffect(() => {
    if (isConfirmed) {
      console.log('Registration successful!', hash);
      showToast({ type: 'success', message: 'Identity registered successfully!' });
      setIsRegisteredForCurrentRound(true);
    }
    if (writeError || confirmationError) {
      console.error('Registration failed:', writeError || confirmationError);
      showToast({ type: 'danger', message: `Registration failed` });
    }
  }, [isConfirmed, writeError, confirmationError, hash, showToast]);

  useEffect(() => {
    if (semaphoreIdentity) {
      return;
    }

    let identity: Identity;
    if (identityPrivateKey) {
      identity = Identity.import(identityPrivateKey);
      console.log('Semaphore identity loaded from storage.');
    } else {
      identity = new Identity();
      setIdentityPrivateKey(identity.export());
      console.log('New Semaphore identity generated and saved.');
    }
    setSemaphoreIdentity(identity);
  }, [identityPrivateKey, setIdentityPrivateKey, semaphoreIdentity]);

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
        logout,
      }}
    >
      {children}
    </VoteManagementContextProvider>
  )
}

export { useVoteManagementContext, VoteManagementProvider }
