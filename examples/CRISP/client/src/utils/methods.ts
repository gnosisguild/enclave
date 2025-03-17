import { PollOption, PollRequestResult, PollResult } from '@/model/poll.model'
import { VoteStateLite } from '@/model/vote.model'

export const markWinner = (options: PollOption[]) => {
  const highestVoteCount = Math.max(...options.map((o) => o.votes))
  return options.map((option) => ({
    ...option,
    checked: option.votes === highestVoteCount,
  }))
}

export const convertTimestampToDate = (timestamp: number, secondsToAdd: number = 0): Date => {
  const date = new Date(timestamp * 1000)
  date.setSeconds(date.getMinutes() + secondsToAdd)
  return date
}

export const hasPollEnded = (pollLength: number, startTime: number): boolean => {
  const endTime = (startTime + pollLength) * 1000
  const currentTime = Date.now()
  return currentTime >= endTime
}

export const hasPollEndedByTimestamp = (endTime: number): boolean => {
  const endTimeMillis = endTime * 1000
  const currentTime = Date.now()
  return currentTime >= endTimeMillis
}

export const formatDate = (isoDateString: string): string => {
  const date = new Date(isoDateString)

  const dateFormatter = new Intl.DateTimeFormat('en-US', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  })

  const timeFormatter = new Intl.DateTimeFormat('en-US', {
    hour: 'numeric',
    minute: 'numeric',
    hour12: true,
  })

  return `${dateFormatter.format(date)} -  ${timeFormatter.format(date)}`
}

export const convertPollData = (request: PollRequestResult[]): PollResult[] => {
  const pollResults = request.map((poll) => {
    const totalVotes = poll.total_votes
    const options: PollOption[] = [
      {
        value: 0,
        votes: poll.option_1_tally,
        label: poll.option_1_emoji,
        checked: false,
      },
      {
        value: 1,
        votes: poll.option_2_tally,
        label: poll.option_2_emoji,
        checked: false,
      },
    ]

    const date = new Date(poll.end_time * 1000).toISOString()

    return {
      endTime: poll.end_time,
      roundId: poll.round_id,
      totalVotes: totalVotes,
      date: date,
      options: options,
    }
  })

  pollResults.sort((a, b) => b.endTime - a.endTime)

  return pollResults
}

export const convertVoteStateLite = (voteState: VoteStateLite): PollResult => {
  const endTime = voteState.expiration
  const date = new Date(endTime * 1000).toISOString()

  const options: PollOption[] = [
    {
      value: 0,
      votes: 0,
      label: voteState.emojis[0],
      checked: false,
    },
    {
      value: 1,
      votes: 0,
      label: voteState.emojis[1],
      checked: false,
    },
  ]

  return {
    roundId: voteState.id,
    totalVotes: voteState.vote_count,
    date: date,
    options: options,
    endTime: endTime,
  }
}

export const debounce = (func: (...args: any[]) => void, wait: number) => {
  let timeout: ReturnType<typeof setTimeout>
  return (...args: any[]) => {
    clearTimeout(timeout)
    timeout = setTimeout(() => func(...args), wait)
  }
}
