import React, { Fragment, useCallback, useEffect, useState } from 'react'
import DailyPollSection from '@/pages/Landing/components/DailyPoll'
import { useVoteManagementContext } from '@/context/voteManagement'
import { convertTimestampToDate } from '@/utils/methods'

const DailyPoll: React.FC = () => {
  const { existNewRound, roundState } = useVoteManagementContext()
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

  return (
    <Fragment>
      <DailyPollSection loading={newRoundLoading} endTime={endTime} />
    </Fragment>
  )
}

export default DailyPoll
