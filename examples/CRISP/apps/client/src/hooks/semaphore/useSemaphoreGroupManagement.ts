import {
    useAccount,
    useReadContract,
    useWriteContract,
    useWaitForTransactionReceipt,
    useWatchContractEvent,
} from 'wagmi';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useNotificationAlertContext } from '@/context/NotificationAlert/NotificationAlert.context';
import {
    E3_PROGRAM_ADDRESS,
    E3_PROGRAM_ABI,
} from '@/config/Enclave.abi';
import { SEMAPHORE_ADDRESS, SEMAPHORE_ABI } from '@/config/Semaphore.abi';
import { SemaphoreEthers } from '@semaphore-protocol/data';
import type { Identity } from '@semaphore-protocol/core/identity';
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

    /* ── 1. groupId ──────────────────────────────────────────────── */
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

        ethersRef.current = new SemaphoreEthers(rpcUrl, {
            address: SEMAPHORE_ADDRESS,
            startBlock: roundStartBlock,
        });
    }, [chain?.id, roundStartBlock]);

    const membersQuery = useQuery<string[]>({
        enabled: !!groupId && !!ethersRef.current,
        queryKey: ['semaphore-members', groupId?.toString()],
        queryFn: async () => {
            const raw = await ethersRef.current!.getGroupMembers(groupId!.toString());
            return raw.map((m: bigint | string) => m.toString());
        },
        staleTime: 1000 * 60 * 5,
    });

    const isFetchingMembers =
        membersQuery.data === undefined || membersQuery.isFetching;
    const groupMembers = membersQuery.data ?? [];

    useWatchContractEvent({
        address: SEMAPHORE_ADDRESS,
        abi: SEMAPHORE_ABI,
        eventName: 'MemberAdded',
        onLogs(logs) {
            if (!groupId) return;

            for (const log of logs as any[]) {
                const evGroupId: bigint = log.args.groupId;
                if (evGroupId !== groupId) continue;

                const commitStr: string = log.args.identityCommitment.toString();
                const key = ['semaphore-members', groupId.toString()];

                queryClient.setQueryData<string[]>(key, (old) =>
                    old?.includes(commitStr) ? old : [...(old ?? []), commitStr],
                );
                queryClient.invalidateQueries({ queryKey: key, exact: true });
            }
        },
    });

    const isCommitted = !!(
        semaphoreIdentity &&
        groupMembers.includes(semaphoreIdentity.commitment.toString())
    );

    console.log('isCommitted', isCommitted);
    console.log('groupMembers', groupMembers);

    const {
        data: txHash,
        writeContract,
        error: writeError,
        isPending: isWritePending,
    } = useWriteContract();

    const { isLoading: isConfirming, isSuccess: isRegConfirmed } =
        useWaitForTransactionReceipt({ hash: txHash });

    const registerIdentity = useCallback(() => {
        if (!roundId || !semaphoreIdentity) {
            showToast({ type: 'danger', message: 'Round or identity missing.' });
            return;
        }
        writeContract({
            address: E3_PROGRAM_ADDRESS,
            abi: E3_PROGRAM_ABI,
            functionName: 'registerMember',
            args: [BigInt(roundId), semaphoreIdentity.commitment],
        });
    }, [roundId, semaphoreIdentity, writeContract, showToast]);

    useEffect(() => {
        if (isRegConfirmed && groupId && semaphoreIdentity) {
            showToast({ type: 'success', message: 'Identity registration confirmed!' });
            const key = ['semaphore-members', groupId.toString()];
            const c = semaphoreIdentity.commitment.toString();
            queryClient.setQueryData<string[]>(key, (old) =>
                old?.includes(c) ? old : [...(old ?? []), c],
            );
        }
        if (writeError) {
            showToast({ type: 'danger', message: 'Registration transaction failed' });
        }
    }, [isRegConfirmed, writeError, queryClient, groupId, semaphoreIdentity]);

    return {
        groupId,
        groupMembers,
        isFetchingMembers,
        isCommitted,
        isRegistering: isWritePending || isConfirming,
        registerIdentity,
    };
};
