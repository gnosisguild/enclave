import { useState, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useVoteManagementContext } from '@/context/voteManagement';
import { useNotificationAlertContext } from '@/context/NotificationAlert/NotificationAlert.context.tsx';
import { Poll } from '@/model/poll.model';
import { BroadcastVoteRequest } from '@/model/vote.model';
import { Group } from '@semaphore-protocol/group';
import { generateNoirProof, SemaphoreNoirProof, initSemaphoreNoirBackend } from '@/utils/semaphoreNoirProof';
import { encodeSemaphoreNoirProof } from '@/utils/proof-encoding';
import { encodeScope } from '@/utils/scopeEncoding';

export const useVoteCasting = () => {
    const {
        user,
        roundState,
        votingRound,
        semaphoreIdentity,
        currentGroupMembers,
        currentSemaphoreGroupId,
        fetchingMembers,
        isRegisteredForCurrentRound,
        encryptVote,
        broadcastVote,
        setTxUrl,
    } = useVoteManagementContext();

    const { showToast } = useNotificationAlertContext();
    const navigate = useNavigate();
    const [isLoading, setIsLoading] = useState<boolean>(false);

    const handleVoteEncryption = useCallback(
        async (vote: Poll) => {
            if (!votingRound) throw new Error('No voting round available for encryption');
            return encryptVote(BigInt(vote.value), new Uint8Array(votingRound.pk_bytes));
        },
        [encryptVote, votingRound],
    );

    const castVoteWithProof = useCallback(async (pollSelected: Poll | null) => {
        if (!pollSelected) {
            console.log("Cannot cast vote: Poll option not selected.");
            showToast({ type: 'danger', message: 'Please select a poll option first.' });
            return;
        }
        if (!user || !isRegisteredForCurrentRound || !roundState || !semaphoreIdentity) {
            console.error("Cannot cast vote: Missing user, registration, round state, or identity.");
            showToast({ type: 'danger', message: 'Cannot cast vote. Ensure you are connected, registered, and the round is active.' });
            return;
        }
        if (fetchingMembers) {
            console.log("Cannot cast vote: Still fetching group members.");
            showToast({ type: 'danger', message: 'Group members are still loading. Please wait.' });
            return;
        }
        if (!currentGroupMembers || currentGroupMembers.length === 0) {
            console.error("Cannot cast vote: No group members found for this round.");
            showToast({ type: 'danger', message: 'Could not load group members for this round.' });
            return;
        }
        if (!currentSemaphoreGroupId) {
            console.error("Cannot cast vote: No group ID available for this round.");
            showToast({ type: 'danger', message: 'Group ID not available for this round.' });
            return;
        }

        setIsLoading(true);
        console.log("Processing vote...");

        try {
            const voteEncrypted = await handleVoteEncryption(pollSelected);
            if (!voteEncrypted) {
                throw new Error("Failed to encrypt vote.");
            }

            // Initialize Noir backend for proof generation
            const merkleTreeDepth = Math.ceil(Math.log2(currentGroupMembers.length)) || 10;
            const backend = await initSemaphoreNoirBackend(merkleTreeDepth);

            const group = new Group(currentGroupMembers);
            
            // CRITICAL FIX: Encode scope properly with address and group ID
            // The scope encodes both the subject address and group ID to prevent front-running attacks:
            // - Upper 160 bits: Subject address (20 bytes)  
            // - Lower 96 bits: Group ID (12 bytes)
            const encodedScope = encodeScope(user.address, currentSemaphoreGroupId);
            const scope = encodedScope.toString();
            const message = String(pollSelected.value);
            
            console.log("Scope encoding details:", {
                userAddress: user.address,
                groupId: currentSemaphoreGroupId.toString(),
                encodedScope: encodedScope.toString(),
                scope
            });
            
            // Generate Noir proof
            const fullProof: SemaphoreNoirProof = await generateNoirProof(
                semaphoreIdentity as any, 
                group, 
                message, 
                scope,
                backend,
                true // Use keccak for Solidity verifier
            );
            
            console.log("Full generated Noir proof object:", fullProof);
            const proofBytes = encodeSemaphoreNoirProof(fullProof);

            const voteRequest: BroadcastVoteRequest = {
                round_id: roundState.id,
                enc_vote_bytes: Array.from(voteEncrypted.vote),
                proof: Array.from(voteEncrypted.proof),
                public_inputs: voteEncrypted.public_inputs,
                address: user.address,
                proof_sem: Array.from(proofBytes)
            };

            console.log("Broadcasting vote to server...");
            const result = await broadcastVote(voteRequest);
            console.log("Vote broadcast result:", result);

            if (result && result.status === 'success') {
                showToast({ type: 'success', message: 'Vote successfully submitted!' });
                if (result.tx_hash) {
                    console.log('Transaction hash:', result.tx_hash);
                    setTxUrl(result.tx_hash);
                }
                navigate('/poll/result');
            } else {
                throw new Error(result?.message || 'Failed to broadcast vote');
            }
        } catch (error) {
            console.error("Error during vote casting:", error);
            const errorMessage = error instanceof Error ? error.message : 'Unknown error occurred';
            showToast({ type: 'danger', message: `Failed to cast vote: ${errorMessage}` });
        } finally {
            setIsLoading(false);
        }
    }, [
        user,
        isRegisteredForCurrentRound,
        roundState,
        semaphoreIdentity,
        fetchingMembers,
        currentGroupMembers,
        currentSemaphoreGroupId,
        handleVoteEncryption,
        broadcastVote,
        showToast,
        setTxUrl,
        navigate,
    ]);

    return {
        castVoteWithProof,
        isLoading,
    };
};