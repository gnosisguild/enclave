// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import {
    useAccount,
    useReadContract,
    useWriteContract,
    useWaitForTransactionReceipt,
} from 'wagmi';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useNotificationAlertContext } from '@/context/NotificationAlert/NotificationAlert.context';
import {
    E3_PROGRAM_ADDRESS,
    E3_PROGRAM_ABI,
} from '@/config/Enclave.abi';
import { SemaphoreEthers } from '@semaphore-protocol/data';
import type { Identity } from '@semaphore-protocol/identity';
import { useCallback, useEffect, useRef } from 'react';

interface SemaphoreGroupManagement {
    groupId: bigint | null;
    groupMembers: string[];
    isFetchingMembers: boolean;
    isRegistering: boolean;
    isCommitted: boolean;
    registerIdentity: () => void;
}

export const useSemaphoreGroupManagement = (
    roundId: number | null | undefined,
    roundStartBlock: number | null | undefined,
    semaphoreIdentity: Identity | null,
): SemaphoreGroupManagement => {
    const { showToast } = useNotificationAlertContext();
    const { chain } = useAccount();
    const queryClient = useQueryClient();

    const { data: rawGroupId } = useReadContract({
        address: roundId != null ? E3_PROGRAM_ADDRESS : undefined,
        abi: roundId != null ? E3_PROGRAM_ABI : undefined,
        functionName: 'groupIds',
        args: roundId != null ? [BigInt(roundId)] : undefined,
        query: { enabled: roundId != null },
    });
    const groupId: bigint | null =
        rawGroupId !== undefined ? (rawGroupId as bigint) : null;

    const ethersRef = useRef<SemaphoreEthers | null>(null);
    useEffect(() => {
        const rpcUrl = chain?.rpcUrls?.default?.http[0];
        if (!rpcUrl || !roundStartBlock) return;
        const semaphoreAddress = import.meta.env.VITE_SEMAPHORE_ADDRESS;
        if (!semaphoreAddress) {
            throw new Error("VITE_SEMAPHORE_ADDRESS environment variable is not set.");
        }
        ethersRef.current = new SemaphoreEthers(rpcUrl, {
            address: semaphoreAddress,
            startBlock: roundStartBlock,
        });
    }, [chain?.id, roundStartBlock]);
    const { data: membersData, isFetching: isFetchingMembers } = useQuery<string[]>({
        enabled: !(groupId == null) && !!ethersRef.current,
        queryKey: ['semaphore-members', groupId?.toString()],
        queryFn: async () => {
            const raw = await ethersRef.current!.getGroupMembers(groupId!.toString());
            return raw.map((m: bigint | string) => m.toString());
        },
        staleTime: 1000 * 60 * 5,
    });
    const groupMembers = membersData ?? [];

    const isCommitted = !!(
        semaphoreIdentity &&
        groupMembers.includes(semaphoreIdentity.commitment.toString())
    );

    const {
        data: txHash,
        writeContract,
        error: writeError,
        isPending: isWritePending,
    } = useWriteContract();

    const { isLoading: isConfirming, isSuccess: isRegConfirmed } =
        useWaitForTransactionReceipt({ hash: txHash });

    const registerIdentity = useCallback(() => {
        if (roundId == null || !semaphoreIdentity) {
            showToast({ type: 'danger', message: 'Round or identity missing.' });
            return;
        }
        writeContract({
            address: E3_PROGRAM_ADDRESS,
            abi: E3_PROGRAM_ABI,
            functionName: 'registerMember',
            args: [BigInt(roundId), semaphoreIdentity.commitment],
        });
    }, [roundId, semaphoreIdentity, writeContract]);

    useEffect(() => {
        if (isRegConfirmed && groupId != null && semaphoreIdentity) {
            showToast({ type: 'success', message: 'Identity registration confirmed!' });
            const key = ['semaphore-members', groupId.toString()];
            const c = semaphoreIdentity.commitment.toString();
            queryClient.setQueryData<string[]>(key, (old) =>
                old?.includes(c) ? old : [...(old ?? []), c],
            );
            queryClient.invalidateQueries({ queryKey: key, exact: true });
        }
        if (writeError) {
            showToast({ type: 'danger', message: 'Registration transaction failed' });
        }
    }, [isRegConfirmed, queryClient, groupId, semaphoreIdentity]);

    return {
        groupId,
        groupMembers,
        isFetchingMembers,
        isCommitted,
        isRegistering: isWritePending || isConfirming,
        registerIdentity,
    };
};
