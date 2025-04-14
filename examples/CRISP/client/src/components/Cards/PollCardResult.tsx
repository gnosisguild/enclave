import React from 'react'
import { PollOption } from '@/model/poll.model'
import Card from '@/components/Cards/Card'
import { ShieldSlash } from '@phosphor-icons/react'

type PollCardResultProps = {
  results: PollOption[]
  totalVotes: number
  spaceCards?: string
  height?: number
  width?: number
  isResult?: boolean
  isActive?: boolean
}
const PollCardResult: React.FC<PollCardResultProps> = ({ isResult, results, totalVotes, isActive }) => {
  const calculatePercentage = (votes: number) => {
    return ((votes / totalVotes) * 100).toFixed(0)
  }

  return (
    <div className={`grid ${isResult ? 'sm:w-full md:w-1/3' : 'w-full'} z-50 grid-cols-2 gap-4 md:gap-8`}>
      {results.map((poll) => (
        <div className='col-span-1 w-full' key={`${poll.label}-${poll.value}`}>
          <div
            className={`flex w-full flex-col items-center justify-center ${isResult ? 'aspect-square space-y-6 max-sm:space-y-2' : 'space-y-4'}`}
          >
            <Card isDetails checked={totalVotes === 0 ? false : poll.checked} isActive={isActive}>
              <p className={isResult ? 'text-8xl max-sm:p-5 max-sm:text-6xl' : 'text-5xl'}>{poll.label}</p>
            </Card>

            {isActive && isResult && (
              <div className={isResult ? 'space-y-2 ' : ''}>
                <div className='flex w-full items-center justify-center space-x-1 rounded-lg border-2 border-slate-600/20 bg-white p-2 py-1 font-bold uppercase  shadow-md'>
                  <ShieldSlash weight='bold' size={24} className='text-slate-800/50' />
                  <div className={`text-center text-sm font-bold uppercase text-slate-800/50 `}>vote encrypted</div>
                </div>
                <p className={` text-center text-sm font-extrabold uppercase text-slate-600/50  max-sm:text-xs`}>revealed when poll ends</p>
              </div>
            )}
            {!isActive && (
              <div className={isResult ? 'space-y-2 max-sm:space-y-0' : ''}>
                <h3
                  className={`text-center ${isResult ? 'text-h1' : 'text-h3'}  font-bold ${totalVotes > 0 && poll.checked ? 'text-lime-400' : 'text-slate-600/50'}`}
                >
                  {totalVotes ? calculatePercentage(poll.votes) : 0}%
                </h3>
                <p className={`text-center ${isResult ? 'text-2xl font-semibold' : 'text-xs font-bold'}  text-slate-600/50`}>
                  {poll.votes} votes
                </p>
              </div>
            )}
          </div>
        </div>
      ))}
    </div>
  )
}

export default PollCardResult
