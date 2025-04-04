import React, { Fragment, useEffect, useState } from 'react'
import CardContent from '@/components/Cards/CardContent'
import VotesBadge from '@/components/VotesBadge'
import PollCardResult from '@/components/Cards/PollCardResult'
import { convertPollData, convertVoteStateLite, formatDate, markWinner } from '@/utils/methods'
import PastPollSection from '@/pages/Landing/components/PastPoll'
import { useParams } from 'react-router-dom'
import LoadingAnimation from '@/components/LoadingAnimation'
import { useVoteManagementContext } from '@/context/voteManagement'
import CircularTiles from '@/components/CircularTiles'
import CountdownTimer from '@/components/CountdownTime'
import ConfirmVote from '../DailyPoll/components/ConfirmVote'

const PollResult: React.FC = () => {
  const params = useParams()
  const { roundId, type } = params
  const { pastPolls, getWebResultByRound, pollResult, setPollResult } = useVoteManagementContext()
  const [loading, setLoading] = useState<boolean>(true)
  const { roundEndDate, txUrl, roundState } = useVoteManagementContext()

  const activeTotalCount = type === 'confirmation' ? roundState?.vote_count : pollResult?.totalVotes

  const fetchPoll = async () => {
    const pollResult = await getWebResultByRound(parseInt(roundId as string))
    if (pollResult) {
      const convertedPoll = convertPollData([pollResult])
      setPollResult(convertedPoll[0])
      setLoading(false)
    }
  }

  useEffect(() => {
    if (!pollResult && roundId && loading) {
      fetchPoll()
    } else if (activeTotalCount && roundState && type === 'confirmation') {
      const currentPoll = convertVoteStateLite(roundState)
      if (currentPoll) {
        setPollResult(currentPoll)
        setLoading(false)
      }
    }
  }, [pastPolls, roundId, roundState, activeTotalCount])

  useEffect(() => {
    if (pollResult && loading) {
      setLoading(false)
    }
  }, [pollResult])

  return (
    <div className='relative flex w-full flex-1 items-center justify-center px-6 py-12'>
      <div className='absolute bottom-0 right-0 grid w-full grid-cols-2 gap-2 max-md:opacity-50 md:w-[70vh]'>
        <CircularTiles count={4} />
      </div>
      <div className='mb-28 flex min-h-[730px] w-screen flex-col  items-center justify-center'>
        {loading && !pollResult && (
          <div className='flex items-center justify-center'>
            <LoadingAnimation isLoading={loading} />
          </div>
        )}
        {!loading && pollResult && (
          <Fragment>
            <div className='my-28 flex w-full flex-col items-center justify-center space-y-12'>
              <div className='flex flex-col items-center justify-center space-y-6'>
                <div className='space-y-2 text-center'>
                  <p className='text-sm font-extrabold uppercase'>Poll {pollResult.roundId}</p>
                  <h1 className='text-h1 font-bold  text-slate-600 max-sm:text-3xl'>
                    {type === 'confirmation' ? 'Thanks for voting!' : 'Poll Results'}
                  </h1>
                  {type !== 'confirmation' && <p className='text-2xl font-bold max-sm:text-lg'>{formatDate(pollResult.date)}</p>}
                </div>
                {type === 'confirmation' && roundEndDate && (
                  <div className='flex items-center justify-center max-sm:py-5 '>
                    <CountdownTimer endTime={roundEndDate} />
                  </div>
                )}
                <VotesBadge totalVotes={activeTotalCount ?? 0} />
              </div>
              <PollCardResult
                results={markWinner(pollResult.options)}
                totalVotes={pollResult.totalVotes}
                isResult
                isActive={type === 'confirmation' ? true : false}
              />
            </div>

            {type === 'confirmation' && txUrl && <ConfirmVote confirmationUrl={txUrl} />}
            {type !== 'confirmation' && (
              <CardContent>
                <div className='space-y-4'>
                  <p className='text-base font-extrabold uppercase text-slate-600/50'>WHAT JUST HAPPENED?</p>
                  <div className='space-y-2'>
                    <p className='text-xl leading-8 text-slate-600 max-sm:text-lg'>
                      After casting your vote, CRISP securely processed your selection using a blend of Fully Homomorphic Encryption (FHE),
                      threshold cryptography, and zero-knowledge proofs (ZKPs), without revealing your identity or choice. Your vote was
                      encrypted and anonymously aggregated with others, ensuring the integrity of the voting process while strictly
                      maintaining confidentiality. The protocol's advanced cryptographic techniques guarantee that your vote contributes to
                      the final outcome without any risk of privacy breaches or undue influence.
                    </p>
                  </div>
                </div>
                <div className='space-y-4'>
                  <p className='text-base font-extrabold uppercase text-slate-600/50'>WHAT DOES THIS MEAN?</p>
                  <p className='text-xl leading-8 text-slate-600 max-sm:text-lg'>
                    Your participation has directly contributed to a transparent and fair decision-making process, showcasing the power of
                    privacy-preserving technology in governance and beyond. The use of CRISP in this vote represents a significant step
                    towards secure, anonymous, and tamper-proof digital elections and polls. This innovation ensures that every vote counts
                    equally while safeguarding against the risks of fraud and coercion, enhancing the reliability and trustworthiness of
                    digital decision-making platforms.
                  </p>
                </div>
              </CardContent>
            )}
            {pastPolls.length > 0 && (
              <div className='z-50'>
                <PastPollSection customLabel='Historic polls' useFullHeight={false} limit={3} />
              </div>
            )}
          </Fragment>
        )}
      </div>
    </div>
  )
}

export default PollResult
