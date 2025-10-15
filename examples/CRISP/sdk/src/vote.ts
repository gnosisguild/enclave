// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { ProtocolParams } from '@enclave-e3/sdk'
import { IVote, VotingMode } from './types'
import { toBinary } from './utils'

/**
 * Encode a vote based on the voting mode
 * @param vote The vote to encode
 * @param votingMode The voting mode to use for encoding
 * @param bfvConfig The BFV protocol parameters to use for encoding
 * @param votingPower The voting power of the voter
 * @returns The encoded vote as a string
 */
export const encodeVote = (vote: IVote, votingMode: VotingMode, bfvConfig: ProtocolParams, votingPower: bigint): string[] => {
  validateVote(votingMode, vote, votingPower)

  switch (votingMode) {
    case VotingMode.GOVERNANCE:
      const voteArray = []
      const length = bfvConfig.degree
      const halfLength = length / 2
      const yesBinary = toBinary(vote.yes).split('')
      const noBinary = toBinary(vote.no).split('')

      // Fill first half with 'yes' binary representation (pad with leading 0s if needed)
      for (let i = 0; i < halfLength; i++) {
        const offset = halfLength - yesBinary.length
        voteArray.push(i < offset ? '0' : yesBinary[i - offset])
      }

      // Fill second half with 'no' binary representation (pad with leading 0s if needed)
      for (let i = 0; i < length - halfLength; i++) {
        const offset = length - halfLength - noBinary.length
        voteArray.push(i < offset ? '0' : noBinary[i - offset])
      }
      return voteArray
    default:
      throw new Error('Unsupported voting mode')
  }
}

/**
 * Validate whether a vote is valid for a given voting mode
 * @param votingMode The voting mode to validate against
 * @param vote The vote to validate
 * @param votingPower The voting power of the voter
 */
export const validateVote = (votingMode: VotingMode, vote: IVote, votingPower: bigint) => {
  switch (votingMode) {
    case VotingMode.GOVERNANCE:
      if (vote.yes > 0n && vote.no > 0n) {
        throw new Error('Invalid vote for GOVERNANCE mode: cannot spread votes between options')
      }

      if (vote.yes > votingPower || vote.no > votingPower) {
        throw new Error('Invalid vote for GOVERNANCE mode: vote exceeds voting power')
      }
  }
}
