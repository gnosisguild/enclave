import React, { useState, useEffect } from 'react'
import { Poll } from '@/model/poll.model'
import Card from '@/components/Cards/Card'
import CircularTiles from '@/components/CircularTiles'

import { useVoteManagementContext } from '@/context/voteManagement'
import LoadingAnimation from '@/components/LoadingAnimation'
import { hasPollEnded } from '@/utils/methods'
import CountdownTimer from '@/components/CountdownTime'
import { useModal } from 'connectkit'
import RegistrationModal from '@/components/RegistrationModal'
import { useVoteCasting } from '@/hooks/voting/useVoteCasting'

type DailyPollSectionProps = {
  loading?: boolean
  endTime: Date | null
}

const DailyPollSection: React.FC<DailyPollSectionProps> = ({ loading, endTime }) => {
  const {
    user,
    pollOptions,
    setPollOptions,
    roundState,
    isRegistering,
    isRegisteredForCurrentRound,
    registerIdentityOnContract,
    fetchingMembers,
  } = useVoteManagementContext()
  const isEnded = roundState ? hasPollEnded(roundState?.duration, roundState?.start_time) : false
  const [pollSelected, setPollSelected] = useState<Poll | null>(null)
  const [noPollSelected, setNoPollSelected] = useState<boolean>(true)
  const { setOpen } = useModal()
  const [showRegistrationModal, setShowRegistrationModal] = useState(false)
  const { castVoteWithProof, isLoading: isCastingVote } = useVoteCasting()

  useEffect(() => {
    if (isRegisteredForCurrentRound && showRegistrationModal) {
      setShowRegistrationModal(false)
    }
  }, [isRegisteredForCurrentRound, showRegistrationModal])

  const statusClass = !isEnded ? 'lime' : 'red'

  const handleChecked = (selectedPoll: Poll) => {
    const isAlreadySelected = pollSelected?.value === selectedPoll.value

    setPollOptions((prevOptions) =>
      prevOptions.map((option) => ({
        ...option,
        checked: option.value === selectedPoll.value ? !isAlreadySelected : false,
      })),
    )

    if (isAlreadySelected) {
      setPollSelected(null)
      setNoPollSelected(true)
    } else {
      setPollSelected(selectedPoll)
      setNoPollSelected(false)
    }
  }

  const castVote = async () => {
    if (!user) {
      setOpen(true)
      return
    }
    if (!isRegisteredForCurrentRound) {
      setShowRegistrationModal(true)
      return
    }

    await castVoteWithProof(pollSelected)
  }

  return (
    <>
      <div className='relative flex w-full flex-1 items-center justify-center px-6 pb-12 pt-20 md:py-12'>
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

          {endTime && !isEnded && !isCastingVote && (
            <div className='flex items-center justify-center max-sm:py-5 '>
              <CountdownTimer endTime={endTime} />
            </div>
          )}
          {isCastingVote && (
            <div className='flex flex-col items-center justify-center space-y-2 max-sm:py-5'>
              <p className='text-base font-bold uppercase text-slate-600/50'>Casting Vote</p>
              <LoadingAnimation isLoading={isCastingVote} />
            </div>
          )}
          {loading && <LoadingAnimation isLoading={loading} />}
          <div className=' grid w-full grid-cols-2 gap-4 md:gap-8'>
            {pollOptions.map((poll) => (
              <div data-test-id={`poll-button-${poll.value}`} key={poll.label} className='col-span-2 md:col-span-1'>
                <Card checked={poll.checked} onChecked={() => handleChecked(poll)}>
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
                disabled={noPollSelected || loading || !roundState || isEnded || isRegistering || isCastingVote || fetchingMembers}
                onClick={castVote}
              >
                {isRegistering
                  ? 'Registering...'
                  : fetchingMembers
                    ? 'Loading Group...'
                    : isCastingVote
                      ? 'Processing Vote...'
                      : 'Cast Vote'}
              </button>
            </div>
          )}
        </div>
      </div>
      <RegistrationModal
        isOpen={showRegistrationModal}
        onClose={() => setShowRegistrationModal(false)}
        isRegistering={isRegistering}
        onRegister={() => {
          registerIdentityOnContract()
        }}
      />
    </>
  )
}

export default DailyPollSection
