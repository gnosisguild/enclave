// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import React, { Fragment, useEffect, useState } from 'react'
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
