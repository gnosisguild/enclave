import { ElegibleVoter } from '@/model/vote.model'

/**
 * Get a random voter details from a list of elegible voters
 * @param addresses The list of elegible voters
 * @returns The randomly selected voter details
 */
export const getRandomVoterToMask = (voters: ElegibleVoter[]): ElegibleVoter => {
  const randomIndex = crypto.getRandomValues(new Uint32Array(1))[0] % voters.length

  return voters[randomIndex]
}
