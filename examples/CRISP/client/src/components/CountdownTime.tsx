// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import React, { useEffect, useState } from 'react'
import LoadingAnimation from '@/components/LoadingAnimation'

interface CountdownTimerProps {
  endTime: Date
}

type RemainingTime = {
  days: string
  hours: string
  minutes: string
  seconds: string
}

const CountdownTimer: React.FC<CountdownTimerProps> = ({ endTime }) => {
  const [remainingTime, setRemainingTime] = useState<RemainingTime | null>(null)
  const [loading, setLoading] = useState<boolean>(true)

  useEffect(() => {
    const timer = setInterval(() => {
      const now = new Date()
      const difference = endTime.getTime() - now.getTime()
      if (difference <= 0) {
        clearInterval(timer)
        setLoading(false)
        setRemainingTime({ days: '0', hours: '0', minutes: '0', seconds: '0' })
        return
      }

      const days = Math.floor(difference / (1000 * 60 * 60 * 24)).toString()
      const hours = Math.floor((difference / (1000 * 60 * 60)) % 24).toString()
      const minutes = Math.floor((difference / 1000 / 60) % 60).toString()
      const seconds = Math.floor((difference / 1000) % 60).toString()
      setRemainingTime({ days, hours, minutes, seconds })
      setLoading(false)
    }, 1000)

    return () => clearInterval(timer)
  }, [endTime])

  return (
    <div className='flex flex-col items-center justify-center space-y-2'>
      <p className='text-base font-bold uppercase text-slate-600/50'>Poll ends in:</p>

      {loading && <LoadingAnimation isLoading={true} />}
      {!loading && remainingTime && (
        <div className='flex space-x-6'>
          <p className='text-2xl font-bold text-slate-600'>
            {remainingTime.days}
            <span className=' text-slate-600/50'>d</span>
          </p>
          <p className='text-2xl font-bold text-slate-600'>
            {remainingTime.hours}
            <span className=' text-slate-600/50'>h</span>
          </p>
          <p className='text-2xl font-bold text-slate-600'>
            {remainingTime.minutes}
            <span className=' text-slate-600/50'>m</span>
          </p>
          <p className='text-2xl font-bold text-slate-600'>
            {remainingTime.seconds}
            <span className=' text-slate-600/50'>s</span>
          </p>
        </div>
      )}
    </div>
  )
}

export default CountdownTimer
