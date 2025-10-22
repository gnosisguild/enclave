// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { ProtocolParams, EnclaveSDK, FheProtocol } from '@enclave-e3/sdk'
import { type CRISPVoteAndInputs, type IVote, VotingMode } from './types'
import { toBinary } from './utils'
import { MAXIMUM_VOTE_VALUE } from './constants'

/**
 * This utility function calculates the first valid index for vote options
 * based on the total voting power and degree.
 * @dev This is needed to calculate the decoded plaintext
 * @dev Also, we will need to check in the circuit that anything within these indices is
 * either 0 or 1.
 * @param totalVotingPower The maximum vote amount (if a single voter had all of the power)
 * @param degree The degree of the polynomial
 */
export const calculateValidIndicesForPlaintext = (totalVotingPower: bigint, degree: number): { yesIndex: number; noIndex: number } => {
  // Sanity check: degree must be even and positive
  if (degree <= 0 || degree % 2 !== 0) {
    throw new Error('Degree must be a positive even number')
  }

  // Calculate the number of bits needed to represent the total voting power
  const bitsNeeded = totalVotingPower.toString(2).length

  const halfLength = Math.floor(degree / 2)

  // Check if bits needed exceed half the degree
  if (bitsNeeded > halfLength) {
    throw new Error('Total voting power exceeds maximum representable votes for the given degree')
  }

  // For "yes": right-align in first half
  // Start index = (half length) - (bits needed)
  const yesIndex = halfLength - bitsNeeded

  // For "no": right-align in second half
  // Start index = (full length) - (bits needed)
  const noIndex = degree - bitsNeeded

  return {
    yesIndex: yesIndex,
    noIndex: noIndex,
  }
}

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
 * Given an encoded tally, decode it into its decimal representation
 * @param tally The encoded tally to decode
 * @param votingMode The voting mode
 * @param bfvConfig The BFV protocol parameters used for encryption
 */
export const decodeTally = (tally: string[], votingMode: VotingMode): IVote => {
  switch (votingMode) {
    case VotingMode.GOVERNANCE:
      const halfLength = tally.length / 2

      // Split the tally into two halves
      const yesBinary = tally.slice(0, halfLength)
      const noBinary = tally.slice(halfLength, tally.length)

      let yes = 0n
      let no = 0n

      // Convert each half back to decimal
      for (let i = 0; i < halfLength; i += 1) {
        const weight = 2n ** BigInt(halfLength - 1 - i)

        yes += BigInt(yesBinary[i]) * weight
        no += BigInt(noBinary[i]) * weight
      }

      return {
        yes,
        no,
      }
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

      if (vote.yes > MAXIMUM_VOTE_VALUE || vote.no > MAXIMUM_VOTE_VALUE) {
        throw new Error('Invalid vote for GOVERNANCE mode: vote exceeds maximum allowed value')
      }
  }
}

/**
 * This is a wrapper around enclave-e3/sdk encryption functions as CRISP circuit will require some more
 * input values which generic Greco do not need.
 * @param encodedVote The encoded vote as string array
 * @param publicKey The public key to use for encryption
 */
export const encryptVoteAndGenerateCRISPInputs = async (encodedVote: string[], publicKey: Uint8Array): Promise<CRISPVoteAndInputs> => {
  // @todo The SDK need refactoring
  const enclaveSDK = EnclaveSDK.create({
    protocol: FheProtocol.BFV,
    chainId: 31337,
    contracts: {
      enclave: '0xc6e7DF5E7b4f2A278906862b61205850344D4e7d',
      ciphernodeRegistry: '0xc6e7DF5E7b4f2A278906862b61205850344D4e7d',
    },
    // local node
    rpcUrl: 'http://localhost:8545',
    // default Anvil private key
    privateKey: '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80',
  })

  // Convert string[] to BigUint64Array
  const bigUint64Array = new BigUint64Array(encodedVote.map((str) => BigInt(str)))

  const encryptedData = await enclaveSDK.encryptVectorAndGenInputs(bigUint64Array, publicKey)

  // the rest of the public and private inputs will need to be generated before calling the circuit to generate the CRISP proof
  return {
    encryptedVote: encryptedData.encryptedData,
    circuitInputs: {
      ...encryptedData.publicInputs,
      // @todo fill the rest of the inputs needed for CRISP
      public_key_x: [],
      public_key_y: [],
      signature: [],
      hashed_message: [],
      balance: '0',
      merkle_proof_length: '0',
      merkle_proof_indices: [],
      merkle_proof_siblings: [],
    },
  }
}
