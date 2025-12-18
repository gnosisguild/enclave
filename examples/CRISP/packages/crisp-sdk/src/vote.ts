// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { ZKInputsGenerator } from '@crisp-e3/zk-inputs'
import { type CircuitInputs, type Vote, ExecuteCircuitResult, MaskVoteProofInputs, ProofInputs, VoteProofInputs } from './types'
import { generateMerkleProof, toBinary, extractSignatureComponents, getAddressFromSignature, getOptimalThreadCount } from './utils'
import { MAXIMUM_VOTE_VALUE, MASK_SIGNATURE, zeroVote, SIGNATURE_MESSAGE_HASH } from './constants'
import { Noir, type CompiledCircuit } from '@noir-lang/noir_js'
import { UltraHonkBackend, type ProofData } from '@aztec/bb.js'
import circuit from '../../../circuits/target/crisp_circuit.json'
import { bytesToHex, encodeAbiParameters, parseAbiParameters, numberToHex, getAddress, hexToBytes } from 'viem/utils'
import { Hex } from 'viem'
import { getIsSlotEmpty, getPreviousCiphertext } from './state'

// Initialize the ZKInputsGenerator.
const zkInputsGenerator: ZKInputsGenerator = ZKInputsGenerator.withDefaults()
const optimalThreadCount = await getOptimalThreadCount()

/**
 * Encode a vote.
 * @param vote The vote to encode.
 * @returns The encoded vote as a BigInt64Array.
 */
export const encodeVote = (vote: Vote): BigInt64Array => {
  const bfvParams = zkInputsGenerator.getBFVParams()
  const voteArray = []
  const length = bfvParams.degree
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

  return BigInt64Array.from(voteArray.map(BigInt))
}

/**
 * Decode bytes to numbers array (little-endian, 8 bytes per value).
 * @param data The bytes to decode (must be multiple of 8).
 * @returns Array of numbers.
 */
const decodeBytesToNumbers = (data: Uint8Array): number[] => {
  if (data.length % 8 !== 0) {
    throw new Error('Data length must be multiple of 8')
  }

  const arrayLength = data.length / 8
  const result: number[] = []

  for (let i = 0; i < arrayLength; i++) {
    const offset = i * 8
    let value = 0

    // Read 8 bytes in little-endian order
    for (let j = 0; j < 8; j++) {
      const byteValue = data[offset + j]
      value |= byteValue << (j * 8)
    }

    result.push(value)
  }

  return result
}

/**
 * Decode an encoded tally into its decimal representation.
 * @param tallyBytes The encoded tally as a hex string (bytes).
 * @returns The decoded tally as an IVote.
 */
export const decodeTally = (tallyBytes: string): Vote => {
  // Convert hex string to bytes, handling both with and without 0x prefix
  const hexString = tallyBytes.startsWith('0x') ? tallyBytes : `0x${tallyBytes}`
  const bytes = hexToBytes(hexString as Hex)

  // Decode bytes to numbers array
  const numbers = decodeBytesToNumbers(bytes)

  const HALF_D = numbers.length / 2

  // Extract the first half for yes votes and second half for no votes
  // Votes are right-aligned with leading zeros, so we can use the entire halves
  const yesBinary = numbers.slice(0, HALF_D)
  const noBinary = numbers.slice(HALF_D, numbers.length)

  let yes = 0n
  let no = 0n

  // Convert yes votes (entire first half)
  for (let i = 0; i < yesBinary.length; i += 1) {
    const weight = 2n ** BigInt(yesBinary.length - 1 - i)
    yes += BigInt(yesBinary[i]) * weight
  }

  // Convert no votes (entire second half)
  for (let i = 0; i < noBinary.length; i += 1) {
    const weight = 2n ** BigInt(noBinary.length - 1 - i)
    no += BigInt(noBinary[i]) * weight
  }

  return {
    yes,
    no,
  }
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
 * Generate the circuit inputs for a vote proof.
 * This works for both vote and masking.
 * @param proofInputs - The proof inputs.
 * @returns The circuit inputs as a CircuitInputs object.
 */
export const generateCircuitInputs = async (proofInputs: ProofInputs): Promise<CircuitInputs> => {
  const encodedVote = encodeVote(proofInputs.vote)

  let crispInputs: CircuitInputs

  if (proofInputs.isFirstVote) {
    crispInputs = await zkInputsGenerator.generateInputs(
      // A placeholder ciphertext vote will be generated.
      // This is safe because the circuit will not check the ciphertext addition if
      // the previous ciphertext is not provided (is_first_vote is true).
      encryptVote(zeroVote, proofInputs.publicKey),
      proofInputs.publicKey,
      encodedVote,
    )
  } else {
    if (!proofInputs.previousCiphertext) {
      throw new Error('Previous ciphertext is required for non-first votes')
    }
    crispInputs = await zkInputsGenerator.generateInputsForUpdate(proofInputs.previousCiphertext, proofInputs.publicKey, encodedVote)
  }

  crispInputs.hashed_message = Array.from(proofInputs.messageHash).map((b) => b.toString())
  crispInputs.public_key_x = Array.from(proofInputs.publicKeyX).map((b) => b.toString())
  crispInputs.public_key_y = Array.from(proofInputs.publicKeyY).map((b) => b.toString())
  crispInputs.signature = Array.from(proofInputs.signature).map((b) => b.toString())
  crispInputs.slot_address = proofInputs.slotAddress.toLowerCase()
  crispInputs.balance = proofInputs.balance.toString()
  crispInputs.is_first_vote = proofInputs.isFirstVote
  crispInputs.merkle_root = proofInputs.merkleProof.proof.root.toString()
  crispInputs.merkle_proof_length = proofInputs.merkleProof.length.toString()
  crispInputs.merkle_proof_indices = proofInputs.merkleProof.indices.map((i) => i.toString())
  crispInputs.merkle_proof_siblings = proofInputs.merkleProof.proof.siblings.map((s) => s.toString())

  return crispInputs
}

/**
 * Execute the circuit.
 * @param crispInputs - The circuit inputs.
 * @returns The execute circuit result.
 */
export const executeCircuit = async (crispInputs: CircuitInputs): Promise<ExecuteCircuitResult> => {
  const noir = new Noir(circuit as CompiledCircuit)

  return noir.execute(crispInputs) as Promise<ExecuteCircuitResult>
}

/**
 * Generate a proof for the CRISP circuit given the circuit inputs.
 * @param crispInputs - The circuit inputs.
 * @returns The proof.
 */
export const generateProof = async (crispInputs: CircuitInputs): Promise<ProofData> => {
  const { witness } = await executeCircuit(crispInputs)
  const backend = new UltraHonkBackend(circuit.bytecode, { threads: optimalThreadCount })

  const proof = await backend.generateProof(witness, { keccakZK: true })

  await backend.destroy()

  return proof
}

/**
 * Generate a vote proof for the CRISP circuit given the vote proof inputs.
 * @param voteProofInputs - The vote proof inputs.
 * @returns The vote proof.
 */
export const generateVoteProof = async (voteProofInputs: VoteProofInputs) => {
  if (voteProofInputs.vote.yes > voteProofInputs.balance || voteProofInputs.vote.no > voteProofInputs.balance) {
    throw new Error('Invalid vote: vote exceeds balance')
  }

  const maxVoteValue = BigInt(MAXIMUM_VOTE_VALUE)
  if (voteProofInputs.vote.yes > maxVoteValue || voteProofInputs.vote.no > maxVoteValue) {
    throw new Error('Invalid vote: vote exceeds maximum allowed value')
  }

  if (voteProofInputs.vote.yes < 0n || voteProofInputs.vote.no < 0n) {
    throw new Error('Invalid vote: vote is negative')
  }

  // The address slot of an actual vote always is the address of the public key that signed the message.
  const address = await getAddressFromSignature(voteProofInputs.signature, voteProofInputs.messageHash)

  const { messageHash, publicKeyX, publicKeyY, signature } = await extractSignatureComponents(
    voteProofInputs.signature,
    voteProofInputs.messageHash,
  )

  const merkleProof = generateMerkleProof(voteProofInputs.balance, address, voteProofInputs.merkleLeaves)

  // check if the slot is empty first
  const isSlotEmpty = await getIsSlotEmpty(voteProofInputs.serverUrl, voteProofInputs.e3Id, voteProofInputs.slotAddress)

  let previousCiphertext: Uint8Array
  if (!isSlotEmpty) {
    previousCiphertext = await getPreviousCiphertext(voteProofInputs.serverUrl, voteProofInputs.e3Id, voteProofInputs.slotAddress)
  } else {
    previousCiphertext = encryptVote(zeroVote, voteProofInputs.publicKey)
  }

  const crispInputs = await generateCircuitInputs({
    ...voteProofInputs,
    slotAddress: address,
    merkleProof,
    isFirstVote: isSlotEmpty,
    previousCiphertext,
    messageHash,
    publicKeyX,
    publicKeyY,
    signature,
  })

  return generateProof(crispInputs)
}

/**
 * Generate a proof for a vote masking operation.
 * @param maskVoteProofInputs The mask vote proof inputs.
 * @returns
 */
export const generateMaskVoteProof = async (maskVoteProofInputs: MaskVoteProofInputs): Promise<ProofData> => {
  // check if the slot is empty first
  const isSlotEmpty = await getIsSlotEmpty(maskVoteProofInputs.serverUrl, maskVoteProofInputs.e3Id, maskVoteProofInputs.slotAddress)

  let previousCiphertext: Uint8Array | undefined

  if (!isSlotEmpty) {
    previousCiphertext = await getPreviousCiphertext(
      maskVoteProofInputs.serverUrl,
      maskVoteProofInputs.e3Id,
      maskVoteProofInputs.slotAddress,
    )
  } else {
    previousCiphertext = encryptVote(zeroVote, maskVoteProofInputs.publicKey)
  }

  const { messageHash, publicKeyX, publicKeyY, signature } = await extractSignatureComponents(MASK_SIGNATURE, SIGNATURE_MESSAGE_HASH)

  const merkleProof = generateMerkleProof(maskVoteProofInputs.balance, maskVoteProofInputs.slotAddress, maskVoteProofInputs.merkleLeaves)

  const crispInputs = await generateCircuitInputs({
    ...maskVoteProofInputs,
    previousCiphertext,
    isFirstVote: isSlotEmpty,
    messageHash,
    publicKeyX,
    publicKeyY,
    signature,
    vote: zeroVote,
    merkleProof,
  })

  return generateProof(crispInputs)
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
export const encodeSolidityProof = (proof: ProofData): Hex => {
  const vote = proof.publicInputs.slice(2) as `0x${string}`[]
  const slotAddress = getAddress(numberToHex(BigInt(proof.publicInputs[0]), { size: 20 }))

  return encodeAbiParameters(parseAbiParameters('bytes, bytes32[], address'), [bytesToHex(proof.proof), vote, slotAddress])
}
