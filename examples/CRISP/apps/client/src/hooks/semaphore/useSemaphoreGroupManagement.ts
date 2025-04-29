import { useState, useEffect, useCallback } from 'react';
import { useReadContract, useWriteContract, useWaitForTransactionReceipt } from 'wagmi';
import { SemaphoreEthers } from '@semaphore-protocol/data';
import { Identity } from '@semaphore-protocol/core/identity';
import { E3_PROGRAM_ADDRESS, E3_PROGRAM_ABI, SEMAPHORE_CONTRACT_ADDRESS } from '@/config/contracts';
import { useNotificationAlertContext } from '@/context/NotificationAlert/NotificationAlert.context.tsx';
import useLocalStorage from '@/hooks/generic/useLocalStorage';

interface SemaphoreGroupManagement {
    groupId: bigint | null;
    groupMembers: string[];
    isFetchingMembers: boolean;
    isRegistering: boolean;
    isRegistered: boolean;
    registerIdentity: () => Promise<void>;
}

export const useSemaphoreGroupManagement = (
    roundId: number | null | undefined,
    semaphoreIdentity: Identity | null
): SemaphoreGroupManagement => {
    const { showToast } = useNotificationAlertContext();
    const [registeredRoundsMap, setRegisteredRoundsMap] = useLocalStorage<Record<string, boolean>>('semaphoreRegisteredRounds', {});
    const [groupId, setGroupId] = useState<bigint | null>(null);
    const [groupMembers, setGroupMembers] = useState<string[]>([]);
    const [isFetchingMembers, setIsFetchingMembers] = useState<boolean>(false);

    // --- Contract Hooks ---
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

    // TODO: Make RPC URL dynamic
    const anvilRpcUrl = 'http://127.0.0.1:8545';
    const [semaphoreEthers] = useState(() =>
        new SemaphoreEthers(anvilRpcUrl, { address: SEMAPHORE_CONTRACT_ADDRESS })
    );

    const isRegistering = isWritePending || isConfirming;
    const isRegistered = roundId !== null && roundId !== undefined ? !!registeredRoundsMap[String(roundId)] : false;

    const fetchGroupMembers = useCallback(async (idToFetch: bigint | string) => {
        setIsFetchingMembers(true);
        console.log(`Fetching members for group: ${idToFetch}`);
        try {
            const members = await semaphoreEthers.getGroupMembers(String(idToFetch));
            setGroupMembers(members);
            console.log('Fetched members:', members);
        } catch (error) {
            console.error('Failed to fetch group members:', error);
            showToast({ type: 'danger', message: 'Could not load group members.' });
            setGroupMembers([]);
        } finally {
            setIsFetchingMembers(false);
        }
    }, [semaphoreEthers, showToast]);

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
    }, [fetchedGroupId, roundId]);

    useEffect(() => {
        if (groupId !== null) {
            fetchGroupMembers(groupId);
        }
    }, [groupId]);

    useEffect(() => {
        if (isRegistrationConfirmed) {
            console.log('Registration successful!', registerHash);
            showToast({ type: 'success', message: 'Identity registered successfully!' });
            if (roundId !== null && roundId !== undefined) {
                setRegisteredRoundsMap(prevMap => ({ ...prevMap, [String(roundId)]: true }));
            }
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
        roundId,
        groupId,
    ]);

    return {
        groupId,
        groupMembers,
        isFetchingMembers,
        isRegistering,
        isRegistered,
        registerIdentity
    };
}; 