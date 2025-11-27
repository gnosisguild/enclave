// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import React, { Fragment, useEffect, useMemo, useState } from 'react'
import { useParams, useNavigate } from 'react-router-dom'
import DailyPollSection from '@/pages/Landing/components/DailyPoll'
import { useVoteManagementContext } from '@/context/voteManagement'
import { convertTimestampToDate } from '@/utils/methods'
import LoadingAnimation from '@/components/LoadingAnimation'

const RoundPoll: React.FC = () => {
  const { roundId } = useParams<{ roundId: string }>()
  const navigate = useNavigate()
  const { roundState, getRoundStateLite, isLoading, currentRoundId } = useVoteManagementContext()
  const [loading, setLoading] = useState(true)

  const parsedRoundId = roundId ? parseInt(roundId, 10) : null

  // If this is the current round, redirect to /current
  useEffect(() => {
    if (parsedRoundId !== null && currentRoundId !== null && parsedRoundId === currentRoundId) {
      navigate('/current', { replace: true })
    }
  }, [parsedRoundId, currentRoundId, navigate])

  // Load the specific round
  useEffect(() => {
    const loadRound = async () => {
      if (parsedRoundId !== null) {
        setLoading(true)
        await getRoundStateLite(parsedRoundId)
        setLoading(false)
      }
    }
    loadRound()
  }, [parsedRoundId, getRoundStateLite])

  const endTime = useMemo(() => (roundState ? convertTimestampToDate(roundState.start_time, roundState.duration) : null), [roundState])

  const title = `Round #${roundId}`

  if (loading || isLoading) {
    return (
      <div className='flex flex-1 items-center justify-center'>
        <LoadingAnimation isLoading />
      </div>
    )
  }

  return (
    <Fragment>
      <DailyPollSection loading={false} endTime={endTime} title={title} />
    </Fragment>
  )
}

export default RoundPoll
