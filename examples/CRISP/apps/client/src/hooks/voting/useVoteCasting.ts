import { useState, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useVoteManagementContext } from '@/context/voteManagement';
import { useNotificationAlertContext } from '@/context/NotificationAlert/NotificationAlert.context.tsx';
import { Poll } from '@/model/poll.model';
import { BroadcastVoteRequest } from '@/model/vote.model';
import { Group, generateProof, SemaphoreProof } from '@semaphore-protocol/core';
import { encodeSemaphoreProof } from '@/utils/proof-encoding';

export const useVoteCasting = () => {
    const {
        user,
        roundState,
        votingRound,
        semaphoreIdentity,
        currentGroupMembers,
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

        setIsLoading(true);
        console.log("Processing vote...");

        try {
            const voteEncrypted = await handleVoteEncryption(pollSelected);
            if (!voteEncrypted) {
                throw new Error("Failed to encrypt vote.");
            }

            const group = new Group(currentGroupMembers);
            const scope = String(roundState.id);
            const message = String(pollSelected.value);
            const fullProof: SemaphoreProof = await generateProof(semaphoreIdentity, group, message, scope);
            console.log("Full generated proof object:", fullProof);
            const proofBytes = encodeSemaphoreProof(fullProof);

            const voteRequest: BroadcastVoteRequest = {
                round_id: roundState.id,
                enc_vote_bytes: Array.from(voteEncrypted.vote),
                proof: Array.from(voteEncrypted.proof),
                public_inputs: voteEncrypted.public_inputs,
                address: user.address,
                proof_sem: Array.from(proofBytes)
            };

            const broadcastVoteResponse = await broadcastVote(voteRequest);
            console.log('broadcastVoteResponse', broadcastVoteResponse)

            if (broadcastVoteResponse) {
                switch (broadcastVoteResponse.status) {
                    case 'success': {
                        const url = `https://sepolia.etherscan.io/tx/${broadcastVoteResponse.tx_hash}`;
                        setTxUrl(url);
                        showToast({
                            type: 'success',
                            message: broadcastVoteResponse.message || 'Successfully voted',
                            linkUrl: url,
                        });
                        navigate(`/result/${roundState.id}/confirmation`);
                        break;
                    }
                    case 'user_already_voted':
                        showToast({
                            type: 'danger',
                            message: broadcastVoteResponse.message || 'User has already voted',
                        });
                        break;
                    case 'failed_broadcast':
                    default:
                        showToast({
                            type: 'danger',
                            message: broadcastVoteResponse.message || 'Error broadcasting the vote',
                        });
                        break;
                }
            } else {
                throw new Error('Received no response after broadcasting vote.');
            }
        } catch (error) {
            console.error("Vote processing failed:", error);
            showToast({ type: 'danger', message: `Vote failed: ${error instanceof Error ? error.message : String(error)}` });
        } finally {
            setIsLoading(false);
        }
    }, [
        user,
        roundState,
        votingRound,
        semaphoreIdentity,
        currentGroupMembers,
        fetchingMembers,
        isRegisteredForCurrentRound,
        encryptVote,
        broadcastVote,
        setTxUrl,
        showToast,
        navigate,
        handleVoteEncryption
    ]);

    return { castVoteWithProof, isLoading };
}; 