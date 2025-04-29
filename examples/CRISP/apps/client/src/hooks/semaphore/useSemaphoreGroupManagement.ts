import { useState, useEffect, useCallback } from 'react';
import { useReadContract, useWriteContract, useWaitForTransactionReceipt } from 'wagmi';
import { Identity } from '@semaphore-protocol/core/identity';
import { E3_PROGRAM_ADDRESS, E3_PROGRAM_ABI } from '@/config/contracts';
import { useNotificationAlertContext } from '@/context/NotificationAlert/NotificationAlert.context.tsx';

interface SemaphoreGroupManagement {
    groupId: bigint | null;
    groupMembers: bigint[];
    isFetchingMembers: boolean;
    isRegistering: boolean;
    isCommitted: boolean;
    registerIdentity: () => Promise<void>;
}

export const useSemaphoreGroupManagement = (
    roundId: number | null | undefined,
    semaphoreIdentity: Identity | null
): SemaphoreGroupManagement => {
    const { showToast } = useNotificationAlertContext();
    const [groupId, setGroupId] = useState<bigint | null>(null);
    const [groupMembers, setGroupMembers] = useState<bigint[]>([]);

    const { data: registerHash, error: writeError, isPending: isWritePending, writeContract, reset: resetRegisterWrite } = useWriteContract();
    const { isLoading: isConfirming, isSuccess: isRegistrationConfirmed, error: confirmationError } = useWaitForTransactionReceipt({ hash: registerHash });

    const { data: fetchedGroupId, refetch: refetchGroupId } = useReadContract({
        address: E3_PROGRAM_ADDRESS,
        abi: E3_PROGRAM_ABI,
        functionName: 'groupIds',
        args: roundId !== null && roundId !== undefined ? [BigInt(roundId)] : undefined,
        query: {
            enabled: roundId !== null && roundId !== undefined,
        },
    });

    const { data: isCommittedOnChain, refetch: refetchIsCommitted } = useReadContract({
        address: E3_PROGRAM_ADDRESS,
        abi: E3_PROGRAM_ABI,
        functionName: 'committed',
        args: (groupId !== null && semaphoreIdentity) ?
            [groupId, semaphoreIdentity.commitment] :
            undefined,
        query: {
            enabled: groupId !== null && !!semaphoreIdentity,
        },
    });

    const { data: fetchedMembers, isLoading: isLoadingMembers, refetch: refetchGroupMembers } = useReadContract({
        address: E3_PROGRAM_ADDRESS,
        abi: E3_PROGRAM_ABI,
        functionName: 'getGroupCommitments',
        args: groupId !== null ? [groupId] : undefined,
        query: {
            enabled: groupId !== null,
        },
    });

    const isRegistering = isWritePending || isConfirming;
    const isCommitted = !!isCommittedOnChain;

    const registerIdentity = useCallback(async () => {
        if (roundId === null || roundId === undefined || !semaphoreIdentity) {
            showToast({ type: 'danger', message: 'Cannot register: Round or identity missing.' });
            return;
        }

        const identityCommitment = semaphoreIdentity.commitment;
        console.log(`Registering commitment: ${identityCommitment} for round: ${roundId}`);
        writeContract({
            address: E3_PROGRAM_ADDRESS,
            abi: E3_PROGRAM_ABI,
            functionName: 'registerMember',
            args: [BigInt(roundId), identityCommitment],
        });
    }, [roundId, semaphoreIdentity, writeContract, showToast]);

    useEffect(() => {
        setGroupId(null);
        setGroupMembers([]);
        resetRegisterWrite();
        if (roundId !== null && roundId !== undefined) {
            console.log(`New round detected (${roundId}), fetching groupId...`);
            refetchGroupId();
        }
    }, [roundId, refetchGroupId, resetRegisterWrite]);

    useEffect(() => {
        if (fetchedGroupId !== undefined && fetchedGroupId !== null) {
            setGroupId(fetchedGroupId as bigint);
            console.log("Fetched Semaphore Group ID:", fetchedGroupId);
        }
    }, [fetchedGroupId]);

    useEffect(() => {
        if (Array.isArray(fetchedMembers)) {
            console.log('Fetched group commitments:', fetchedMembers);
            setGroupMembers(Array.from(fetchedMembers));
        } else {
            setGroupMembers([]);
        }
    }, [fetchedMembers]);

    useEffect(() => {
        if (isRegistrationConfirmed) {
            console.log('Registration successful!', registerHash);
            showToast({ type: 'success', message: 'Identity registered successfully!' });
            refetchIsCommitted();
            refetchGroupMembers();
        }
        if (writeError || confirmationError) {
            console.error('Registration failed:', writeError || confirmationError);
            showToast({ type: 'danger', message: "Registration failed" });
        }
    }, [
        isRegistrationConfirmed,
        writeError,
        confirmationError,
        registerHash,
        refetchIsCommitted,
        refetchGroupMembers
    ]);

    return {
        groupId,
        groupMembers,
        isFetchingMembers: isLoadingMembers,
        isRegistering,
        isCommitted,
        registerIdentity
    };
}; 