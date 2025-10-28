// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { ZKInputsGenerator } from '@enclave/crisp-zk-inputs'
import { BFVParams, type CRISPCircuitInputs, type IVote, VotingMode } from './types'
import { toBinary } from './utils'
import { MAXIMUM_VOTE_VALUE, DEFAULT_BFV_PARAMS } from './constants'
import { extractSignature } from './signature'

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
 * @param votingPower The voting power of the voter
 * @param bfvParams The BFV parameters to use for encoding
 * @returns The encoded vote as a string
 */
export const encodeVote = (vote: IVote, votingMode: VotingMode, votingPower: bigint, bfvParams?: BFVParams): string[] => {
  validateVote(votingMode, vote, votingPower)

  switch (votingMode) {
    case VotingMode.GOVERNANCE:
      const voteArray = []
      const length = bfvParams?.degree || DEFAULT_BFV_PARAMS.degree
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
 * @param previousCiphertext The previous ciphertext to use for addition operation
 * @param bfvParams The BFV parameters to use for encryption
 * @returns The CRISP circuit inputs
 */
export const encryptVoteAndGenerateCRISPInputs = async (
  encodedVote: string[],
  publicKey: Uint8Array,
  previousCiphertext: Uint8Array,
  bfvParams: BFVParams = DEFAULT_BFV_PARAMS,
): Promise<CRISPCircuitInputs> => {
  if (encodedVote.length !== bfvParams.degree) {
    throw new RangeError(`encodedVote length ${encodedVote.length} does not match BFV degree ${bfvParams.degree}`)
  }

  const zkInputsGenerator: ZKInputsGenerator = new ZKInputsGenerator(bfvParams.degree, bfvParams.plaintextModulus, bfvParams.moduli)

  const vote = BigInt64Array.from(encodedVote.map(BigInt))

  const crispInputs = (await zkInputsGenerator.generateInputs(previousCiphertext, publicKey, vote)) as CRISPCircuitInputs

  // the rest of the public and private inputs will need to be generated before calling the circuit to generate the CRISP proof
  return {
    ...crispInputs,
    // @todo fill the rest of the inputs needed for CRISP
    public_key_x: [],
    public_key_y: [],
    signature: [],
    hashed_message: [],
    balance: '0',
    merkle_proof_length: '0',
    merkle_proof_indices: [],
    merkle_proof_siblings: [],
  }
}

/**
 * Generate the CRISP circuit inputs by extracting signature components and adding them to the partial inputs
 * @todo Add the merkle tree inputs too
 * @param partialInputs The partial CRISP circuit inputs
 * @param signature The voter's signature
 * @param message The signed message
 * @returns The complete CRISP circuit inputs
 */
export const generateCRISPInputs = async (
  partialInputs: CRISPCircuitInputs,
  signature: `0x${string}`,
  message: string,
): Promise<CRISPCircuitInputs> => {
  const { hashed_message, pub_key_x, pub_key_y, signature: extractedSignature } = await extractSignature(message, signature)

  return {
    ...partialInputs,
    hashed_message: Array.from(hashed_message).map((b) => b.toString()),
    public_key_x: Array.from(pub_key_x).map((b) => b.toString()),
    public_key_y: Array.from(pub_key_y).map((b) => b.toString()),
    signature: Array.from(extractedSignature).map((b) => b.toString()),
  }
}
