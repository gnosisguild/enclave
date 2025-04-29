// src/hooks/semaphore/useSemaphoreProof.ts
import { Identity } from "@semaphore-protocol/identity";
import { generateProof } from "@semaphore-protocol/proof";
import { SemaphoreEthers } from "@semaphore-protocol/data";
import { Group } from "@semaphore-protocol/group"
import { handleGenericError } from '@/utils/handle-generic-error';
import { useApi } from '../generic/useFetchApi';
import { GroupIdResponse } from '@/model/vote.model';

const ENCLAVE_API = import.meta.env.VITE_ENCLAVE_API;

if (!ENCLAVE_API) handleGenericError('useSemaphoreProof', { name: 'ENCLAVE_API', message: 'Missing env VITE_ENCLAVE_API' });

const SemaphoreEndpoints = {
    GetGroupId: `${ENCLAVE_API}/rounds/group`,
} as const;

export const useSemaphoreProof = () => {
    const { fetchData, isLoading } = useApi();

    const getGroupIdForRound = () =>
        fetchData<GroupIdResponse>(SemaphoreEndpoints.GetGroupId);

    const createVoteProof = async (
        identity: Identity,
        roundId: number,
        voteValue: number
    ): Promise<string | undefined> => {
        try {
            // Get group ID for this round
            const groupIdResponse = await getGroupIdForRound();
            if (!groupIdResponse || !groupIdResponse.exists) {
                handleGenericError('createVoteProof', {
                    name: 'GroupIdError',
                    message: 'No group exists for this round'
                });
                return undefined;
            }

            const groupId = groupIdResponse.group_id;

            // Initialize SemaphoreEthers with the contract address
            const semaphoreEthers = new SemaphoreEthers("http://localhost:8545", {
                address: "0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0"
            });


            const members = await semaphoreEthers.getGroupMembers(groupId);

            const group = new Group(members);

            // Make sure our identity is in the group
            const identityCommitment = identity.commitment;
            const index = group.indexOf(identityCommitment);

            if (index === -1) {
                handleGenericError('createVoteProof', {
                    name: 'MembershipError',
                    message: 'Identity not in group'
                });
                return undefined;
            }

            // Generate proof
            const signal = voteValue.toString();
            const externalNullifier = roundId.toString();
            const fullProof = await generateProof(identity, group, signal, externalNullifier);

            // Serialize proof for the server
            return "0x" + Buffer.from(JSON.stringify(fullProof)).toString('hex');
        } catch (error) {
            handleGenericError('createVoteProof', error as Error);
            return undefined;
        }
    };

    return {
        isLoading,
        createVoteProof,
        getGroupIdForRound
    };
};