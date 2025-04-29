import React, { Fragment, useCallback, useEffect, useState } from 'react'
import DailyPollSection from '@/pages/Landing/components/DailyPoll'
import { Poll } from '@/model/poll.model'
//import  {SemaphoreProof} from "@/model/vote.model.ts";
import { useVoteManagementContext } from '@/context/voteManagement'
import { useNotificationAlertContext } from '@/context/NotificationAlert'
import { useNavigate } from 'react-router-dom'
import { convertTimestampToDate } from '@/utils/methods'
import { createIdentityFromWallet } from '@/utils/identityUtils';
import { useSemaphoreProof } from '@/hooks/semaphore/useSemaphoreProof';

const DailyPoll: React.FC = () => {
  const navigate = useNavigate()
  const { showToast } = useNotificationAlertContext()
  const { encryptVote, broadcastVote, getRoundStateLite, existNewRound, setTxUrl, votingRound, roundState, user } =
      useVoteManagementContext()
  const { createVoteProof } = useSemaphoreProof();
  const [voteCasting, setVoteCasting] = useState<boolean>(false)
  const [newRoundLoading, setNewRoundLoading] = useState<boolean>(false)
  const endTime = roundState && convertTimestampToDate(roundState?.start_time, roundState?.duration)

  useEffect(() => {
    const checkRound = async () => {
      setNewRoundLoading(true)
      await existNewRound()
    }
    checkRound()
  }, [])

  useEffect(() => {
    if (roundState) {
      setNewRoundLoading(false)
    }
  }, [roundState])

  const handleVoteEncryption = useCallback(
      async (vote: Poll) => {
        if (!votingRound) throw new Error('No voting round available')
        return encryptVote(BigInt(vote.value), new Uint8Array(votingRound.pk_bytes))
      },
      [encryptVote, votingRound],
  )

  const handleVoteBroadcast = useCallback(
      async (voteEncrypted: Uint8Array, semaphoreProof?: any) => {
        if (!user || !votingRound) throw new Error('User or voting round not available')
        console.log('user', user, 'votingRound', votingRound)
        const res = await broadcastVote({
          round_id: votingRound.round_id,
          enc_vote_bytes: Array.from(voteEncrypted),
          address: user.address ?? '',
          proof_sem: semaphoreProof,
        })
        console.log('res', res)
        return res
      },
      [broadcastVote, user, votingRound],
  )
  const handleVoted = async (vote: Poll | null) => {
    if (!vote || !votingRound) return
    setVoteCasting(true)

    try {
      //Creating identity again for the purpose of providing input to the generateVoteProof function
      const identity = await createIdentityFromWallet(votingRound.round_id);

      let semaphoreProof;
      try {
        semaphoreProof = await createVoteProof(identity, votingRound.round_id, vote.value);
        if (!semaphoreProof) {
          throw new Error("Failed to generate zero-knowledge proof");
        }
      } catch (proofError) {
        console.error("Error generating proof:", proofError);
        showToast({ type: 'danger', message: 'Error generating zero-knowledge proof' });
        setVoteCasting(false);
        return;
      }
      const voteEncrypted = await handleVoteEncryption(vote)
      console.log('voteEncrypted', voteEncrypted)
      const broadcastVoteResponse = voteEncrypted && (await handleVoteBroadcast(voteEncrypted,semaphoreProof))

      await getRoundStateLite(votingRound.round_id)

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
            navigate(`/result/${votingRound.round_id}/confirmation`);
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
        showToast({ type: 'danger', message: 'Error broadcasting the vote' })
      }
    } catch (error) {
      console.error('Error handling vote:', error)
      showToast({ type: 'danger', message: 'Error processing the vote' })
    } finally {
      setVoteCasting(false)
    }
  }

  return (
      <Fragment>
        <DailyPollSection onVoted={handleVoted} loading={newRoundLoading} voteCasting={voteCasting} endTime={endTime} />
      </Fragment>
  )
}

export default DailyPoll
