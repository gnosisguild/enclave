export interface PollOption {
  value: number
  votes: number
  label: string // emoji
  checked?: boolean
}

export interface PollResult {
  roundId: number
  totalVotes: number
  date: string
  options: PollOption[]
  endTime: number
}

export interface PollRequestResult {
  round_id: number
  option_1_tally: number
  option_2_tally: number
  option_1_emoji: string
  option_2_emoji: string
  end_time: number
  total_votes: number
}

export interface Poll {
  value: number
  checked: boolean
  label: string
}

export interface PollEmoji {
  round_id: number
  emojis: [string, string]
}
