import React, { useEffect, useState } from 'react'
import { Poll } from '@/model/poll.model'
import Card from '@/components/Cards/Card'
import Modal from '@/components/Modal'
import CircularTiles from '@/components/CircularTiles'

import { useVoteManagementContext } from '@/context/voteManagement'
import LoadingAnimation from '@/components/LoadingAnimation'
import { hasPollEnded } from '@/utils/methods'
import CountdownTimer from '@/components/CountdownTime'
import { useSignIn } from '@farcaster/auth-kit'

import FarcasterModal from '@/components/FarcasterModal'

type DailyPollSectionProps = {
  onVoted?: (vote: Poll) => void
  loading?: boolean
  voteCasting?: boolean
  endTime: Date | null
}

const DailyPollSection: React.FC<DailyPollSectionProps> = ({ onVoted, loading, voteCasting, endTime }) => {
  const { url, connect, signIn, data, error } = useSignIn({
    timeout: 300000,
    interval: 2000,
  })
  const { user, pollOptions, setPollOptions, roundState } = useVoteManagementContext()
  const isEnded = roundState ? hasPollEnded(roundState?.duration, roundState?.start_time) : false
  const status = roundState?.status
  const [pollSelected, setPollSelected] = useState<Poll | null>(null)
  const [noPollSelected, setNoPollSelected] = useState<boolean>(true)
  const [modalOpen, setModalOpen] = useState(false)

  const openModal = () => setModalOpen(true)
  const closeModal = () => {
    setModalOpen(false)
  }

  useEffect(() => {
    const fetch = async () => {
      await connect()
    }
    fetch()
  }, [])

  const handleChecked = (selectedId: number) => {
    const updatedOptions = pollOptions.map((option) => ({
      ...option,
      checked: !option.checked && option.value === selectedId,
    }))
    setPollSelected(updatedOptions.find((opt) => opt.checked) ?? null)
    setPollOptions(updatedOptions)
    setNoPollSelected(updatedOptions.every((poll) => !poll.checked))
  }

  const castVote = () => {
    if (!user) {
      signIn()
      return openModal()
    }
    if (pollSelected && onVoted) {
      onVoted(pollSelected)
    }
  }

  const statusClass = !isEnded ? 'lime' : 'red'

  return (
    <>
      <div className='relative flex w-full flex-1 items-center justify-center px-6 py-12'>
        <div className='absolute bottom-0 right-0 grid w-full grid-cols-2 gap-2 max-md:opacity-50 md:w-[70vh]'>
          <CircularTiles count={4} />
        </div>

        <div className='relative mx-auto flex w-full max-w-screen-md flex-col items-center justify-center space-y-8'>
          <div className='space-y-2'>
            <p className='text-center text-sm font-extrabold uppercase text-slate-400'>Daily Poll</p>
            <h3 className='md:text-h3 text-center font-bold leading-none text-slate-600'>Choose your favorite</h3>
            {!roundState && <p className='text-center text-2xl font-bold text-slate-600/50 '>There are is no current daily poll.</p>}
          </div>
          {roundState && (
            <div className='flex items-center justify-center space-x-2'>
              <div
                className={`flex items-center space-x-2 rounded-lg border-2 border-${statusClass}-600/80 ${!isEnded ? 'bg-lime-400' : 'bg-red-400'} px-2 py-1 text-center font-bold uppercase leading-none text-white`}
              >
                <div className='h-1.5 w-1.5 animate-pulse rounded-full bg-white'></div>
                <div>{!isEnded ? 'Live' : 'Ended'}</div>
              </div>
              <div className='rounded-lg border-2 border-slate-600/20 bg-white px-2 py-1.5 text-center font-bold uppercase leading-none text-slate-800/50'>
                {roundState.vote_count} votes
              </div>
            </div>
          )}

          {endTime && !isEnded && !voteCasting && (
            <div className='flex items-center justify-center max-sm:py-5 '>
              <CountdownTimer endTime={endTime} />
            </div>
          )}
          {voteCasting && (
            <div className='flex flex-col items-center justify-center max-sm:py-5 space-y-2'>
              <p className='text-base font-bold uppercase text-slate-600/50'>Casting Vote</p>
              <LoadingAnimation isLoading={voteCasting} />
            </div>
          )}
          {loading && (<LoadingAnimation isLoading={loading} />)}
          <div className=' grid w-full grid-cols-2 gap-4 md:gap-8'>
            {pollOptions.map((poll) => (
              <div key={poll.label} className='col-span-2 md:col-span-1'>
                <Card checked={poll.checked} onChecked={() => handleChecked(poll.value)}>
                  <p className='inline-block text-6xl leading-none md:text-8xl'>{poll.label}</p>
                </Card>
              </div>
            ))}
          </div>
          {roundState && (
            <div className='space-y-4'>
              {noPollSelected && !isEnded && <div className='text-center text-sm leading-none text-slate-500'>Select your favorite</div>}
              <button
                className={`button-outlined button-max ${noPollSelected ? 'button-disabled' : ''}`}
                disabled={noPollSelected || loading || status !== 'Active' || isEnded}
                onClick={castVote}
              >
                cast vote
              </button>
            </div>
          )}
        </div>
      </div>
      <Modal show={modalOpen} onClose={closeModal} className='max-w-96 py-12'>
        <FarcasterModal url={url} data={data} error={error} onClose={closeModal} />
      </Modal>
    </>
  )
}

export default DailyPollSection
