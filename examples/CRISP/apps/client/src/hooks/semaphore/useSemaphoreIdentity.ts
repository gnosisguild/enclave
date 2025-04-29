// src/hooks/semaphore/useSemaphoreIdentity.ts
import { Identity } from "@semaphore-protocol/identity"
import { handleGenericError } from '@/utils/handle-generic-error'
import { SemaphoreRegistrationRequest, SemaphoreRegistrationResponse } from '@/model/vote.model'
import { useApi } from '../generic/useFetchApi'
import { createIdentityFromWallet } from '@/utils/identityUtils';
import { useVoteManagementContext } from '@/context/voteManagement';

const ENCLAVE_API = import.meta.env.VITE_ENCLAVE_API

if (!ENCLAVE_API) handleGenericError('useSemaphoreIdentity', { name: 'ENCLAVE_API', message: 'Missing env VITE_ENCLAVE_API' })

const SemaphoreEndpoints = {
    RegisterIdentity: `${ENCLAVE_API}/rounds/register`,
} as const

export const useSemaphoreIdentity = () => {
    const { fetchData, isLoading } = useApi()
    const { votingRound } = useVoteManagementContext();
    const getIdentityFromWallet = async (): Promise<Identity | undefined> => {
        if (!votingRound?.round_id) {
            handleGenericError('useSemaphoreIdentity', { name: 'RoundIDError', message: 'No current voting round ID available' });
            return undefined;
        }

        // Explicitly use votingRound.round_id from context
        const identity = await createIdentityFromWallet(votingRound.round_id);
        return identity;
    };

    const registerWithSemaphoreGroup = (request: SemaphoreRegistrationRequest) =>
        fetchData<SemaphoreRegistrationResponse, SemaphoreRegistrationRequest>(
            SemaphoreEndpoints.RegisterIdentity,
            'post',
            request
        )

    return {
        isLoading,
        getIdentityFromWallet,
        registerWithSemaphoreGroup,
    }
}