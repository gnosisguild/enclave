// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { ZKInputsGenerator } from '@crisp-e3/zk-inputs'
import {
  type CircuitInputs,
  type Vote,
  ExecuteCircuitResult,
  MaskVoteProofInputs,
  ProofInputs,
  ProofData,
  VoteProofInputs,
  Polynomial,
} from './types'
import {
  generateMerkleProof,
  toBinary,
  extractSignatureComponents,
  getAddressFromSignature,
  getOptimalThreadCount,
  getZeroVote,
  getMaxVoteValue,
} from './utils'
import { MASK_SIGNATURE, MAX_VOTE_BITS, SIGNATURE_MESSAGE_HASH } from './constants'
import { Noir, type CompiledCircuit } from '@noir-lang/noir_js'
import { UltraHonkBackend } from '@aztec/bb.js'
import circuit from '../../../circuits/target/crisp_circuit.json'
import { bytesToHex, encodeAbiParameters, parseAbiParameters, numberToHex, getAddress, hexToBytes } from 'viem/utils'
import { Hex } from 'viem'

// Initialize the ZKInputsGenerator.
const zkInputsGenerator: ZKInputsGenerator = ZKInputsGenerator.withDefaults()
const optimalThreadCount = await getOptimalThreadCount()

/**
 * Encode a vote with n choices.
 * @param vote Array of vote values, one per choice.
 * @returns The encoded vote as a BigInt64Array.
 */
export const encodeVote = (vote: Vote): BigInt64Array => {
  if (vote.length < 2) {
    throw new Error('Vote must have at least two choices')
  }

  const bfvParams = zkInputsGenerator.getBFVParams()
  const degree = bfvParams.degree
  const n = vote.length

  // Each choice gets floor(degree/n) bits; remaining bits stay zero
  const segmentSize = Math.floor(degree / n)
  const voteArray: string[] = []

  for (let choiceIdx = 0; choiceIdx < n; choiceIdx += 1) {
    const value = choiceIdx < vote.length ? vote[choiceIdx] : 0n
    const binary = toBinary(value).split('')

    if (binary.length > segmentSize) {
      throw new Error(`Vote value for choice ${choiceIdx} exceeds segment size (${segmentSize} bits)`)
    }

    // Fill segment with binary representation (pad with leading 0s)
    for (let i = 0; i < segmentSize; i += 1) {
      const offset = segmentSize - binary.length
      voteArray.push(i < offset ? '0' : binary[i - offset])
    }
  }

  // Fill remaining bits with zeros
  const remainder = degree - segmentSize * n
  for (let i = 0; i < remainder; i++) {
    voteArray.push('0')
  }

  return BigInt64Array.from(voteArray.map(BigInt))
}

/**
 * Decode bytes to bigint array (little-endian, 8 bytes per value).
 * Uses BigInt to prevent precision loss for u64 values exceeding 2^53-1.
 * @param data The bytes to decode (must be multiple of 8).
 * @returns Array of bigints.
 */
const decodeBytesToBigInts = (data: Uint8Array): bigint[] => {
  if (data.length % 8 !== 0) {
    throw new Error('Data length must be multiple of 8')
  }

  const view = new DataView(data.buffer, data.byteOffset, data.byteLength)
  const arrayLength = data.length / 8
  const result: bigint[] = []

  for (let i = 0; i < arrayLength; i++) {
    result.push(view.getBigUint64(i * 8, true)) // true = little-endian
  }

  return result
}

/**
 * Decode an encoded tally into vote counts for n choices.
 * @param tallyBytes The encoded tally as a hex string.
 * @param numChoices Number of choices.
 * @returns Array of vote counts per choice.
 */
export const decodeTally = (tallyBytes: string, numChoices: number): Vote => {
  const hexString = tallyBytes.startsWith('0x') ? tallyBytes : `0x${tallyBytes}`
  const bytes = hexToBytes(hexString as Hex)
  const values = decodeBytesToBigInts(bytes)

  if (numChoices <= 0) {
    throw new Error('Number of choices must be positive')
  }

  const segmentSize = Math.floor(values.length / numChoices)
  const effectiveSize = Math.min(segmentSize, MAX_VOTE_BITS)
  const results: Vote = []

  for (let choiceIdx = 0; choiceIdx < numChoices; choiceIdx++) {
    const segmentStart = choiceIdx * segmentSize
    const readStart = segmentStart + segmentSize - effectiveSize
    const segment = values.slice(readStart, readStart + effectiveSize)

    let value = 0n
    for (let i = 0; i < segment.length; i++) {
      const weight = 1n << BigInt(segment.length - 1 - i)
      value += segment[i] * weight
    }

    results.push(value)
  }

  return results
}

/**
 * Encrypt the vote using the public key.
 * @param vote - The vote to encrypt.
 * @param publicKey - The public key to use for encryption.
 * @returns The encrypted vote as a Uint8Array.
 */
export const encryptVote = (vote: Vote, publicKey: Uint8Array): Uint8Array => {
  const encodedVote = encodeVote(vote)

  return zkInputsGenerator.encryptVote(publicKey, encodedVote)
}

/**
 * Generate a random public key.
 * @returns The generated public key as a Uint8Array.
 */
export const generatePublicKey = (): Uint8Array => {
  return zkInputsGenerator.generatePublicKey()
}

/**
 * Compute the commitment to a set of ciphertext polynomials.
 * This function is used for testing purposes only. It is not part of the public API.
 * @param ct0is - The first component of the ciphertext polynomials.
 * @param ct1is - The second component of the ciphertext polynomials.
 * @returns The commitment as a bigint.
 */
export const computeCiphertextCommitment = (ct0is: Polynomial[], ct1is: Polynomial[]): bigint => {
  const commitment = zkInputsGenerator.computeCiphertextCommitment(
    ct0is.map((p) => p.coefficients),
    ct1is.map((p) => p.coefficients),
  )

  return BigInt(commitment)
}

/**
 * Generate the circuit inputs for a vote proof.
 * This works for both vote and masking.
 * @param proofInputs - The proof inputs.
 * @returns The circuit inputs as a CircuitInputs object and the encrypted vote as a Uint8Array.
 */
export const generateCircuitInputs = async (
  proofInputs: ProofInputs,
): Promise<{ crispInputs: CircuitInputs; encryptedVote: Uint8Array }> => {
  const numOptions = proofInputs.vote.length
  const zeroVote = getZeroVote(numOptions)
  const encodedVote = encodeVote(proofInputs.vote)

  let crispInputs: CircuitInputs
  let encryptedVote: Uint8Array

  if (!proofInputs.previousCiphertext) {
    const result = await zkInputsGenerator.generateInputs(
      // A placeholder ciphertext vote will be generated.
      // This is safe because the circuit will not check the ciphertext addition if
      // the previous ciphertext is not provided (is_first_vote is true).
      encryptVote(zeroVote, proofInputs.publicKey),
      proofInputs.publicKey,
      encodedVote,
    )

    crispInputs = result.inputs
    encryptedVote = result.encryptedVote
  } else {
    const result = await zkInputsGenerator.generateInputsForUpdate(proofInputs.previousCiphertext, proofInputs.publicKey, encodedVote)

    crispInputs = result.inputs
    encryptedVote = result.encryptedVote
  }

  const signature = await extractSignatureComponents(proofInputs.signature, proofInputs.messageHash)

  crispInputs.hashed_message = Array.from(signature.messageHash).map((b) => b.toString())
  crispInputs.public_key_x = Array.from(signature.publicKeyX).map((b) => b.toString())
  crispInputs.public_key_y = Array.from(signature.publicKeyY).map((b) => b.toString())
  crispInputs.signature = Array.from(signature.signature).map((b) => b.toString())
  crispInputs.slot_address = proofInputs.slotAddress.toLowerCase()
  crispInputs.balance = proofInputs.balance.toString()
  crispInputs.is_first_vote = !proofInputs.previousCiphertext
  crispInputs.is_mask_vote = proofInputs.isMaskVote
  crispInputs.merkle_root = proofInputs.merkleProof.proof.root.toString()
  crispInputs.merkle_proof_length = proofInputs.merkleProof.length.toString()
  crispInputs.merkle_proof_indices = proofInputs.merkleProof.indices.map((i) => i.toString())
  crispInputs.merkle_proof_siblings = proofInputs.merkleProof.proof.siblings.map((s) => s.toString())
  crispInputs.num_options = numOptions.toString()

  return { crispInputs, encryptedVote }
}

/**
 * Execute the circuit.
 * @param crispInputs - The circuit inputs.
 * @returns The execute circuit result.
 */
export const executeCircuit = async (crispInputs: CircuitInputs): Promise<ExecuteCircuitResult> => {
  const noir = new Noir(circuit as CompiledCircuit)

  const { witness, returnValue } = await noir.execute(crispInputs)

  return { witness, returnValue: BigInt(returnValue as `0x${string}`) }
}

/**
 * Generate a proof for the CRISP circuit given the circuit inputs.
 * @param crispInputs - The circuit inputs.
 * @returns The proof.
 */
export const generateProof = async (crispInputs: CircuitInputs) => {
  const { witness } = await executeCircuit(crispInputs)
  const backend = new UltraHonkBackend(circuit.bytecode, { threads: optimalThreadCount })

  const proof = await backend.generateProof(witness, { keccakZK: true })

  await backend.destroy()

  return proof
}

/**
 * Validate a vote.
 * @param vote - The vote to validate.
 * @param balance - The balance of the voter.
 */
export const validateVote = (vote: Vote, balance: bigint): void => {
  const numChoices = vote.length
  const maxValue = getMaxVoteValue(numChoices)

  for (let i = 0; i < vote.length; i++) {
    if (vote[i] < 0n) {
      throw new Error(`Invalid vote: choice ${i} is negative`)
    }
    if (vote[i] > maxValue) {
      throw new Error(`Invalid vote: choice ${i} exceeds maximum encodable value`)
    }
  }

  if (numChoices === 2) {
    // Binary: mutually exclusive
    const nonZeroCount = vote.filter((v) => v > 0n).length
    if (nonZeroCount > 1) {
      throw new Error('Invalid vote: for 2 options, only one choice can be non-zero')
    }
    const votedAmount = vote.find((v) => v > 0n) ?? 0n
    if (votedAmount > balance) {
      throw new Error('Invalid vote: vote exceeds balance')
    }
  } else {
    // 3+ options: split allowed, total capped
    const total = vote.reduce((sum, v) => sum + v, 0n)
    if (total > balance) {
      throw new Error(`Invalid vote: total votes (${total}) exceed balance (${balance})`)
    }
  }
}

/**
 * Generate a vote proof for the CRISP circuit given the vote proof inputs.
 * @param voteProofInputs - The vote proof inputs.
 * @returns The vote proof.
 */
export const generateVoteProof = async (voteProofInputs: VoteProofInputs): Promise<ProofData> => {
  // first validate the vote
  validateVote(voteProofInputs.vote, voteProofInputs.balance)

  const address = await getAddressFromSignature(voteProofInputs.signature, voteProofInputs.messageHash)

  const merkleProof = generateMerkleProof(voteProofInputs.balance, address, voteProofInputs.merkleLeaves)

  const { crispInputs, encryptedVote } = await generateCircuitInputs({
    ...voteProofInputs,
    slotAddress: address,
    merkleProof,
    previousCiphertext: voteProofInputs.previousCiphertext,
    signature: voteProofInputs.signature,
    messageHash: voteProofInputs.messageHash,
    isMaskVote: false,
  })

  return { ...(await generateProof(crispInputs)), encryptedVote }
}

/**
 * Generate a proof for a vote masking operation.
 * @param maskVoteProofInputs The mask vote proof inputs.
 * @returns
 */
export const generateMaskVoteProof = async (maskVoteProofInputs: MaskVoteProofInputs): Promise<ProofData> => {
  const merkleProof = generateMerkleProof(maskVoteProofInputs.balance, maskVoteProofInputs.slotAddress, maskVoteProofInputs.merkleLeaves)

  const { crispInputs, encryptedVote } = await generateCircuitInputs({
    ...maskVoteProofInputs,
    signature: MASK_SIGNATURE,
    messageHash: SIGNATURE_MESSAGE_HASH,
    vote: getZeroVote(maskVoteProofInputs.numOptions),
    merkleProof,
    isMaskVote: true,
  })

  return { ...(await generateProof(crispInputs)), encryptedVote }
}

/**
 * Locally verify a Noir proof.
 * @param proof - The proof to verify.
 * @returns True if the proof is valid, false otherwise.
 */
export const verifyProof = async (proof: ProofData): Promise<boolean> => {
  const backend = new UltraHonkBackend((circuit as CompiledCircuit).bytecode, { threads: optimalThreadCount })

  const isValid = await backend.verifyProof(proof, { keccakZK: true })

  await backend.destroy()

  return isValid
}

/**
 * Encode the proof data into a format that can be used by the CRISP program in Solidity
 * to validate the proof.
 * @param proof The proof data.
 * @returns The encoded proof data as a hex string.
 */
export const encodeSolidityProof = ({ publicInputs, proof, encryptedVote }: ProofData): Hex => {
  const slotAddress = getAddress(numberToHex(BigInt(publicInputs[3]), { size: 20 }))
  const encryptedVoteCommitment = publicInputs[6] as `0x${string}`

  return encodeAbiParameters(parseAbiParameters('bytes, address, bytes32, bytes'), [
    bytesToHex(proof),
    slotAddress,
    encryptedVoteCommitment,
    bytesToHex(encryptedVote),
  ])
}
