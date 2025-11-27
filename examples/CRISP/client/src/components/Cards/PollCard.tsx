// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import React, { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { PollOption, PollResult } from '@/model/poll.model'
import VotesBadge from '@/components/VotesBadge'
import PollCardResult from '@/components/Cards/PollCardResult'
import { formatDate, hasPollEndedByTimestamp, markWinner } from '@/utils/methods'
import { useVoteManagementContext } from '@/context/voteManagement'

const PollCard: React.FC<PollResult> = ({ roundId, options, totalVotes, date, endTime }) => {
  const navigate = useNavigate()
  const [results, setResults] = useState<PollOption[]>(options)
  const [isActive, setIsActive] = useState(!hasPollEndedByTimestamp(endTime))
  const { roundState, setPollResult, currentRoundId } = useVoteManagementContext()

  const isCurrentRound = roundId === currentRoundId
  const displayVoteCount = isCurrentRound && isActive ? (roundState?.vote_count ?? totalVotes) : totalVotes

  useEffect(() => {
    if (!isActive) return

    const checkPollStatus = () => {
      const pollEnded = hasPollEndedByTimestamp(endTime)
      if (pollEnded) {
        setIsActive(false)
      }
    }

    checkPollStatus()
    const interval = setInterval(checkPollStatus, 1000)

    return () => clearInterval(interval)
  }, [endTime, isActive])

  useEffect(() => {
    const newPollOptions = markWinner(options)
    setResults(newPollOptions)
  }, [options])

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
      className='relative flex min-h-[248px] w-full cursor-pointer flex-col items-center justify-center space-y-4 rounded-3xl border-2 border-slate-600/20 bg-white/50 p-8 pt-2 shadow-lg md:max-w-[274px] hover:border-slate-600/40 transition-colors'
      onClick={handleNavigation}
    >
      <div className='external-icon absolute right-4 top-4' />
      <div className='text-xs font-bold text-slate-600'>{formatDate(date)}</div>
      <div className='flex space-x-8 '>
        <PollCardResult results={results} totalVotes={displayVoteCount} isActive={isActive} />
      </div>
      {isActive && (
        <div
          className={`flex items-center space-x-2 rounded-lg border-2 ${isCurrentRound ? 'border-lime-600/80 bg-lime-400' : 'border-blue-600/80 bg-blue-400'} px-2 py-1 text-center font-bold uppercase leading-none text-white`}
        >
          <div className='h-1.5 w-1.5 animate-pulse rounded-full bg-white'></div>
          <div>{isCurrentRound ? 'Live' : 'Active'}</div>
        </div>
      )}
      <div className='absolute bottom-[-1rem] left-1/2 -translate-x-1/2 transform '>
        <VotesBadge totalVotes={displayVoteCount} />
      </div>
    </div>
  )
}

export default PollCard
