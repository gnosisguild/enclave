// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import React, { Fragment, useMemo } from 'react'
import DailyPollSection from '@/pages/Landing/components/DailyPoll'
import { useVoteManagementContext } from '@/context/voteManagement'
import { convertTimestampToDate } from '@/utils/methods'

const DailyPoll: React.FC = () => {
  const { roundState, isLoading } = useVoteManagementContext()
  const endTime = useMemo(() => (roundState ? convertTimestampToDate(roundState.start_time, roundState.duration) : null), [roundState])

  const loading = isLoading || !roundState

  return (
    <Fragment>
      <DailyPollSection loading={loading} endTime={endTime} />
    </Fragment>
  )
}

export default DailyPoll
