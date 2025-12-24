// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { EligibleVoter } from '@/model/vote.model'

/**
 * Get a random voter details from a list of eligible voters
 * @param addresses The list of eligible voters
 * @returns The randomly selected voter details
 */
export const getRandomVoterToMask = (voters: EligibleVoter[]): EligibleVoter => {
  if (voters.length === 0) {
    throw new Error('No eligible voters available to select from.')
  }

  const randomIndex = crypto.getRandomValues(new Uint32Array(1))[0] % voters.length

  return voters[randomIndex]
}
