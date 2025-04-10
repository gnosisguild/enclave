import { Poll, PollEmoji } from '@/model/poll.model'

export const generatePoll = (poll: PollEmoji): Poll[] => {
  const { emojis } = poll
  return [
    {
      value: 0,
      label: emojis[0],
      checked: false,
    },
    { value: 1, label: emojis[1], checked: false },
  ]
}
