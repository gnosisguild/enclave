// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { ZKInputsGenerator } from '@crisp-e3/zk-inputs'
import { BFVParams, type CRISPCircuitInputs, type EncryptVoteAndGenerateCRISPInputsParams, type IVote, VotingMode } from './types'
import { toBinary } from './utils'
import { MAXIMUM_VOTE_VALUE, DEFAULT_BFV_PARAMS, HALF_LARGEST_MINIMUM_DEGREE, MESSAGE } from './constants'
import { extractSignature } from './signature'
import { Noir, type CompiledCircuit } from '@noir-lang/noir_js'
import { UltraHonkBackend, type ProofData } from '@aztec/bb.js'
import circuit from '../../../circuits/target/crisp_circuit.json'
import { privateKeyToAccount } from 'viem/accounts'

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
      const HALF_D = tally.length / 2
      const START_INDEX_Y = HALF_D - HALF_LARGEST_MINIMUM_DEGREE
      const START_INDEX_N = tally.length - HALF_LARGEST_MINIMUM_DEGREE

      // Extract only the relevant parts of the tally
      const yesBinary = tally.slice(START_INDEX_Y, HALF_D)
      const noBinary = tally.slice(START_INDEX_N, tally.length)

      let yes = 0n
      let no = 0n

      // Convert yes votes (from START_INDEX_Y to HALF_D)
      for (let i = 0; i < yesBinary.length; i += 1) {
        const weight = 2n ** BigInt(yesBinary.length - 1 - i)
        yes += BigInt(yesBinary[i]) * weight
      }

      // Convert no votes (from START_INDEX_N to D)
      for (let i = 0; i < noBinary.length; i += 1) {
        const weight = 2n ** BigInt(noBinary.length - 1 - i)
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
 * @param merkleData The merkle proof data
 * @param message The message that was signed
 * @param signature The signature of the message
 * @param balance The voter's balance
 * @param slotAddress The voter's slot address
 * @param isFirstVote Whether this is the first vote for this slot
 * @returns The CRISP circuit inputs
 */
export const encryptVoteAndGenerateCRISPInputs = async ({
  encodedVote,
  publicKey,
  previousCiphertext,
  bfvParams = DEFAULT_BFV_PARAMS,
  merkleData,
  message,
  signature,
  balance,
  slotAddress,
  isFirstVote,
}: EncryptVoteAndGenerateCRISPInputsParams): Promise<CRISPCircuitInputs> => {
  if (encodedVote.length !== bfvParams.degree) {
    throw new RangeError(`encodedVote length ${encodedVote.length} does not match BFV degree ${bfvParams.degree}`)
  }

  const zkInputsGenerator: ZKInputsGenerator = new ZKInputsGenerator(bfvParams.degree, bfvParams.plaintextModulus, bfvParams.moduli)

  const vote = BigInt64Array.from(encodedVote.map(BigInt))

  const crispInputs = (await zkInputsGenerator.generateInputs(previousCiphertext, publicKey, vote)) as CRISPCircuitInputs

  const { hashed_message, pub_key_x, pub_key_y, signature: extractedSignature } = await extractSignature(message, signature)

  return {
    ...crispInputs,
    hashed_message: Array.from(hashed_message).map((b) => b.toString()),
    public_key_x: Array.from(pub_key_x).map((b) => b.toString()),
    public_key_y: Array.from(pub_key_y).map((b) => b.toString()),
    signature: Array.from(extractedSignature).map((b) => b.toString()),
    merkle_proof_length: merkleData.length.toString(),
    merkle_proof_indices: merkleData.indices.map((i) => i.toString()),
    merkle_proof_siblings: merkleData.proof.siblings.map((s) => s.toString()),
    merkle_root: merkleData.proof.root.toString(),
    slot_address: slotAddress,
    balance: balance.toString(),
    is_first_vote: isFirstVote,
  }
}

/**
 * A function to generate the data required to mask a vote
 * @param voter The voter's address
 * @param publicKey The voter's public key
 * @param previousCiphertext The previous ciphertext
 * @param bfvParams The BFV parameters
 * @param merkleRoot The merkle root of the census tree
 * @param slotAddress The voter's slot address
 * @param isFirstVote Whether this is the first vote for this slot
 * @returns The CRISP circuit inputs for a mask vote
 */
export const generateMaskVote = async (
  publicKey: Uint8Array,
  previousCiphertext: Uint8Array,
  bfvParams = DEFAULT_BFV_PARAMS,
  merkleRoot: bigint,
  slotAddress: string,
  isFirstVote: boolean,
): Promise<CRISPCircuitInputs> => {
  const plaintextVote: IVote = {
    yes: 0n,
    no: 0n,
  }

  const encodedVote = encodeVote(plaintextVote, VotingMode.GOVERNANCE, 0n, bfvParams)

  const zkInputsGenerator: ZKInputsGenerator = new ZKInputsGenerator(bfvParams.degree, bfvParams.plaintextModulus, bfvParams.moduli)

  const vote = BigInt64Array.from(encodedVote.map(BigInt))

  const crispInputs = (await zkInputsGenerator.generateInputs(previousCiphertext, publicKey, vote)) as CRISPCircuitInputs

  // hardhat default private key
  const privateKey = '0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80'
  const account = privateKeyToAccount(privateKey)
  const signature = await account.signMessage({ message: MESSAGE })
  const { hashed_message, pub_key_x, pub_key_y, signature: extractedSignature } = await extractSignature(MESSAGE, signature)

  return {
    ...crispInputs,
    hashed_message: Array.from(hashed_message).map((b) => b.toString()),
    public_key_x: Array.from(pub_key_x).map((b) => b.toString()),
    public_key_y: Array.from(pub_key_y).map((b) => b.toString()),
    signature: Array.from(extractedSignature).map((b) => b.toString()),
    merkle_proof_indices: Array.from({ length: 20 }, () => '0'),
    merkle_proof_siblings: Array.from({ length: 20 }, () => '0'),
    merkle_proof_length: '1',
    merkle_root: merkleRoot.toString(),
    slot_address: slotAddress,
    balance: '0',
    is_first_vote: isFirstVote,
  }
}

export const generateProof = async (crispInputs: CRISPCircuitInputs): Promise<ProofData> => {
  const noir = new Noir(circuit as CompiledCircuit)
  const backend = new UltraHonkBackend((circuit as CompiledCircuit).bytecode)

  const { witness } = await noir.execute(crispInputs as any)
  const proof = await backend.generateProof(witness, { keccak: true })

  return proof
}

export const generateProofWithReturnValue = async (
  crispInputs: CRISPCircuitInputs,
): Promise<{ returnValue: unknown; proof: ProofData }> => {
  const noir = new Noir(circuit as CompiledCircuit)
  const backend = new UltraHonkBackend((circuit as CompiledCircuit).bytecode)

  const { witness, returnValue } = await noir.execute(crispInputs as any)
  const proof = await backend.generateProof(witness, { keccak: true })

  return { returnValue, proof }
}

export const getCircuitOutputValue = async (crispInputs: CRISPCircuitInputs): Promise<{ returnValue: unknown }> => {
  const noir = new Noir(circuit as CompiledCircuit)

  const { returnValue } = await noir.execute(crispInputs as any)

  return { returnValue }
}

export const verifyProof = async (proof: ProofData): Promise<boolean> => {
  const backend = new UltraHonkBackend((circuit as CompiledCircuit).bytecode)

  return await backend.verifyProof(proof)
}
