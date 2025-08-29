// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import React, { useCallback, useEffect, useState } from 'react'
import PollCard from '@/components/Cards/PollCard'
import { PollResult } from '@/model/poll.model'
import LoadingAnimation from '@/components/LoadingAnimation'
import { useVoteManagementContext } from '@/context/voteManagement'
import CircularTiles from '@/components/CircularTiles'
import { debounce } from '@/utils/methods'

const HistoricPoll: React.FC = () => {
  const { votingRound, pastPolls, getPastPolls, isLoading } = useVoteManagementContext()
  const [visiblePolls, setVisiblePolls] = useState<PollResult[]>([])
  const [page, setPage] = useState<number>(0)
  const [loadingMore, setLoadingMore] = useState<boolean>(false)

  const loadMorePolls = useCallback(() => {
    if (loadingMore || isLoading) return
    setLoadingMore(true)
    setTimeout(() => {
      setPage((prevPage) => prevPage + 1)
      window.scrollTo({
        top: document.documentElement.scrollTop - 150,
        behavior: 'smooth',
      })
      setLoadingMore(false)
    }, 1000) // 1 second delay
  }, [loadingMore, isLoading])

  useEffect(() => {
    if (votingRound && votingRound?.pk_bytes) {
      const fetchPastPolls = async () => {
        await getPastPolls()
      }
      fetchPastPolls()
    }
  }, [votingRound])

  useEffect(() => {
    setVisiblePolls(pastPolls.slice(0, 12)) // Initialize with the first 12 polls
  }, [pastPolls])

  useEffect(() => {
    const newVisiblePolls = pastPolls.slice(0, (page + 1) * 12)
    setVisiblePolls(newVisiblePolls)
  }, [page, pastPolls])

  const handleScroll = useCallback(
    debounce(() => {
      const { scrollTop, clientHeight, scrollHeight } = document.documentElement
      if (scrollTop + clientHeight >= scrollHeight && !loadingMore && pastPolls.length > visiblePolls.length) {
        loadMorePolls()
      }
    }, 200),
    [loadMorePolls, loadingMore, pastPolls.length, visiblePolls.length],
  )

  useEffect(() => {
    window.addEventListener('scroll', handleScroll)
    return () => window.removeEventListener('scroll', handleScroll)
  }, [handleScroll])

  return (
    <div className='relative mt-8 flex w-full flex-1 items-center justify-center px-6 py-12 md:mt-0'>
      <div className='absolute bottom-0 right-0 grid w-full grid-cols-2 gap-2 max-md:opacity-50 md:w-[70vh]'>
        <CircularTiles count={4} />
      </div>
      <div className='relative mx-auto flex w-full flex-col items-center justify-center space-y-8'>
        <h1 className='text-h1 mt-20 font-bold text-slate-600'>Historic polls</h1>
        {isLoading && (
          <div className='flex justify-center'>
            <LoadingAnimation isLoading={isLoading} />
          </div>
        )}
        {!pastPolls.length && !isLoading && <p className=' text-2xl font-bold text-slate-600/50 '>There are no historic polls.</p>}
        {visiblePolls.length > 0 && (
          <div className='mx-auto grid w-full max-w-7xl grid-cols-1 items-center gap-8 overflow-y-auto p-4 md:grid-cols-3'>
            {visiblePolls.map((pollResult: PollResult, index: number) => {
              return (
                <div
                  data-test-id={`poll-${pollResult.roundId}-${index}`}
                  className='flex items-center justify-center'
                  key={`${pollResult.roundId}-${index}`}
                >
                  <PollCard {...pollResult} />
                </div>
              )
            })}
          </div>
        )}
        {loadingMore && (
          <div className='flex w-full items-center justify-center'>
            <LoadingAnimation isLoading={loadingMore} />
          </div>
        )}
      </div>
    </div>
  )
}

export default HistoricPoll
