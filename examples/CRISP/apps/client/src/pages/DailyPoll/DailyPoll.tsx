import React, { Fragment, useCallback, useEffect, useState } from 'react'
import DailyPollSection from '@/pages/Landing/components/DailyPoll'
import { Poll } from '@/model/poll.model'
import { useVoteManagementContext } from '@/context/voteManagement'
import { useNotificationAlertContext } from '@/context/NotificationAlert'
import { useNavigate } from 'react-router-dom'
import { convertTimestampToDate } from '@/utils/methods'

const DailyPoll: React.FC = () => {
  const navigate = useNavigate()
  const { showToast } = useNotificationAlertContext()
  const { encryptVote, broadcastVote, getRoundStateLite, existNewRound, setTxUrl, votingRound, roundState, user } =
    useVoteManagementContext()
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
    async (voteEncrypted: Uint8Array) => {
      if (!user || !votingRound) throw new Error('User or voting round not available')
      console.log('user', user, 'votingRound', votingRound)
      const res = await broadcastVote({
        round_id: votingRound.round_id,
        enc_vote_bytes: Array.from(voteEncrypted),
        address: user.address ?? '',
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
      const voteEncrypted = await handleVoteEncryption(vote)
      console.log('voteEncrypted', voteEncrypted)
      const broadcastVoteResponse = voteEncrypted && (await handleVoteBroadcast(voteEncrypted))

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
