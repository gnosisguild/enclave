import { VotingConfigRequest } from '@/model/vote.model'
import { Chain } from '@/utils/network'

//"0x51Ec8aB3e53146134052444693Ab3Ec53663a12B" e.g votingAddress
export const generateCrispRound = (votingAddress: string): VotingConfigRequest => {
  return {
    round_id: 0, // We can get this from the server
    chain_id: Chain.SEPOLIA,
    voting_address: votingAddress,
    ciphernode_count: 2, // We can hard code this so they don't have to choose
    voter_count: 0, // The server will replace this with a timestamp for how long they have to vote
  }
}