// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import React, { useEffect, useMemo, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { PollResult } from '@/model/poll.model'
import VotesBadge from '@/components/VotesBadge'
import PollCardResult from '@/components/Cards/PollCardResult'
import { formatDate, markWinner } from '@/utils/methods'
import { useVoteManagementContext } from '@/context/voteManagement'
import { usePublicClient } from 'wagmi'

const PollCard: React.FC<PollResult> = ({ roundId, options, totalVotes, date, endTime }) => {
  const navigate = useNavigate()
  const [isActive, setIsActive] = useState(true)
  const { roundState, setPollResult, currentRoundId } = useVoteManagementContext()
  const client = usePublicClient()

  const isCurrentRound = roundId === currentRoundId
  const displayVoteCount = isCurrentRound && isActive ? (roundState?.vote_count ?? totalVotes) : totalVotes

  useEffect(() => {
    if (!isActive || !client) return

    const checkPollStatus = async () => {
      const block = await client.getBlock()

      if (block.timestamp >= endTime) {
        setIsActive(false)
      }
    }

    checkPollStatus()
    const interval = setInterval(checkPollStatus, 1000)

    return () => clearInterval(interval)
  }, [endTime, client, isActive])

  const results = useMemo(() => markWinner(options), [options])

  const handleNavigation = () => {
    if (isActive && isCurrentRound) {
      return navigate('/current')
    }
    if (isActive && !isCurrentRound) {
      return navigate(`/round/${roundId}`)
    }
    navigate(`/result/${roundId}`)
    setPollResult({
      roundId,
      options,
      totalVotes,
      date,
      endTime,
    })
  }

  return (
    <div
      className='card col'
      style={{ width: '100%', maxWidth: 300, gap: 16, cursor: 'pointer', alignItems: 'center' }}
      onClick={handleNavigation}
    >
      <div className='between' style={{ width: '100%' }}>
        <span className='mono-sm muted'>{formatDate(date)}</span>
        <span className='linkish' style={{ border: 'none', padding: 0 }}>
          View →
        </span>
      </div>
      <PollCardResult results={results} totalVotes={displayVoteCount} isActive={isActive} />
      <div className='row' style={{ gap: 8, justifyContent: 'center' }}>
        {isActive && <span className={`tag dot ${isCurrentRound ? 'live' : 'tally'}`}>{isCurrentRound ? 'Live' : 'Active'}</span>}
        <VotesBadge totalVotes={displayVoteCount} />
      </div>
    </div>
  )
}

export default PollCard
