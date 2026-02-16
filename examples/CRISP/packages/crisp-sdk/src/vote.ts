// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { ZKInputsGenerator } from '@crisp-e3/zk-inputs'
import {
  type CRISPCircuitInputs,
  type Vote,
  ExecuteCircuitResult,
  MaskVoteProofInputs,
  ProofInputs,
  ProofData,
  VoteProofInputs,
  Polynomial,
  GrecoCircuitInputs,
} from './types'
import {
  generateMerkleProof,
  toBinary,
  extractSignatureComponents,
  getAddressFromSignature,
  getOptimalThreadCount,
  getZeroVote,
  getMaxVoteValue,
  decodeBytesToNumbers,
  numberArrayToBigInt64Array,
} from './utils'
import { MASK_SIGNATURE, MAX_VOTE_BITS, SIGNATURE_MESSAGE_HASH } from './constants'
import { Noir, type CompiledCircuit } from '@noir-lang/noir_js'
import { deflattenFields, UltraHonkBackend } from '@aztec/bb.js'
import crispCircuit from '../../../circuits/bin/crisp/target/crisp.json'
import grecoCircuit from '../../../circuits/bin/greco/target/crisp_greco.json'
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
export const encodeVote = (vote: Vote): number[] => {
  if (vote.length < 2) {
    throw new Error('Vote must have at least two choices')
  }

  const bfvParams = zkInputsGenerator.getBFVParams()
  const degree = bfvParams.degree
  const n = vote.length

  // Each choice gets floor(degree/n) bits; remaining bits stay zero
  const segmentSize = Math.floor(degree / n)
  const maxBits = Math.min(segmentSize, MAX_VOTE_BITS)
  const maxValue = (1 << maxBits) - 1
  const voteArray: number[] = []

  for (let choiceIdx = 0; choiceIdx < n; choiceIdx += 1) {
    const value = choiceIdx < vote.length ? vote[choiceIdx] : 0

    if (value > maxValue) {
      throw new Error(`Vote value for choice ${choiceIdx} exceeds maximum (${maxValue})`)
    }

    const binary = toBinary(value).split('')

    for (let i = 0; i < segmentSize; i += 1) {
      const offset = segmentSize - binary.length
      voteArray.push(i < offset ? 0 : parseInt(binary[i - offset]))
    }
  }

  const remainder = degree - segmentSize * n
  for (let i = 0; i < remainder; i++) {
    voteArray.push(0)
  }

  return voteArray
}

/**
 * Decode an encoded tally into vote counts for n choices.
 * @param tallyBytes The encoded tally as a hex string.
 * @param numChoices Number of choices.
 * @returns Array of vote counts per choice.
 */
export const decodeTally = (tallyBytes: string | number[], numChoices: number): Vote => {
  if (typeof tallyBytes === 'string') {
    // Convert hex string to bytes, handling both with and without 0x prefix
    // and decode to numbers.
    const hexString = tallyBytes.startsWith('0x') ? tallyBytes : `0x${tallyBytes}`
    tallyBytes = decodeBytesToNumbers(hexToBytes(hexString as Hex))
  }

  if (numChoices <= 0) {
    throw new Error('Number of choices must be positive')
  }

  const segmentSize = Math.floor(tallyBytes.length / numChoices)
  const effectiveSize = Math.min(segmentSize, MAX_VOTE_BITS)
  const results: Vote = []

  for (let choiceIdx = 0; choiceIdx < numChoices; choiceIdx++) {
    const segmentStart = choiceIdx * segmentSize
    const readStart = segmentStart + segmentSize - effectiveSize
    const segment = tallyBytes.slice(readStart, readStart + effectiveSize)

    let value = 0
    for (let i = 0; i < segment.length; i++) {
      const weight = 2 ** (segment.length - 1 - i)
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

  return zkInputsGenerator.encryptVote(publicKey, numberArrayToBigInt64Array(encodedVote))
}

/**
 * Decrypt the vote using the secret key.
 * @param vote - The vote to decrypt.
 * @param secretKey - The secret key to use for decryption.
 * @param numChoices - The number of choices.
 * @returns The decrypted vote as a Vote.
 */
export const decryptVote = (ciphertext: Uint8Array, secretKey: Uint8Array, numChoices: number): Vote => {
  const decryptedVote = zkInputsGenerator.decryptVote(secretKey, ciphertext)

  return decodeTally(
    Array.from(decryptedVote).map((v) => Number(v)),
    numChoices,
  )
}

/**
 * Generate a random BFV public/secret key pair.
 * @returns The generated public/secret key pair as a JavaScript object.
 */
export const generateBFVKeys = (): { secretKey: Uint8Array; publicKey: Uint8Array } => {
  return zkInputsGenerator.generateKeys()
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
export const generateCircuitInputs = async (proofInputs: ProofInputs): Promise<{ circuitInputs: any; encryptedVote: Uint8Array }> => {
  const numOptions = proofInputs.vote.length
  const zeroVote = getZeroVote(numOptions)
  const encodedVote = encodeVote(proofInputs.vote)

  let circuitInputs: any
  let encryptedVote: Uint8Array

  if (!proofInputs.previousCiphertext) {
    const result = await zkInputsGenerator.generateInputs(
      // A placeholder ciphertext vote will be generated.
      // This is safe because the circuit will not check the ciphertext addition if
      // the previous ciphertext is not provided (is_first_vote is true).
      encryptVote(zeroVote, proofInputs.publicKey),
      proofInputs.publicKey,
      numberArrayToBigInt64Array(encodedVote),
    )

    circuitInputs = result.inputs
    encryptedVote = result.encryptedVote
  } else {
    const result = await zkInputsGenerator.generateInputsForUpdate(
      proofInputs.previousCiphertext,
      proofInputs.publicKey,
      numberArrayToBigInt64Array(encodedVote),
    )

    circuitInputs = result.inputs
    encryptedVote = result.encryptedVote
  }

  const signature = await extractSignatureComponents(proofInputs.signature, proofInputs.messageHash)

  circuitInputs.hashed_message = Array.from(signature.messageHash).map((b) => b.toString())
  circuitInputs.public_key_x = Array.from(signature.publicKeyX).map((b) => b.toString())
  circuitInputs.public_key_y = Array.from(signature.publicKeyY).map((b) => b.toString())
  circuitInputs.signature = Array.from(signature.signature).map((b) => b.toString())
  circuitInputs.slot_address = proofInputs.slotAddress.toLowerCase()
  circuitInputs.balance = proofInputs.balance.toString()
  circuitInputs.is_first_vote = !proofInputs.previousCiphertext
  circuitInputs.is_mask_vote = proofInputs.isMaskVote
  circuitInputs.merkle_root = proofInputs.merkleProof.proof.root.toString()
  circuitInputs.merkle_proof_length = proofInputs.merkleProof.length.toString()
  circuitInputs.merkle_proof_indices = proofInputs.merkleProof.indices.map((i) => i.toString())
  circuitInputs.merkle_proof_siblings = proofInputs.merkleProof.proof.siblings.map((s) => s.toString())
  circuitInputs.num_options = numOptions.toString()

  return { circuitInputs, encryptedVote }
}

/**
 * Execute the circuit.
 * @param crispInputs - The circuit inputs.
 * @returns The execute circuit result.
 */
export const executeCrispCircuit = async (crispInputs: CRISPCircuitInputs): Promise<ExecuteCircuitResult<bigint>> => {
  const noir = new Noir(crispCircuit as CompiledCircuit)

  const { witness, returnValue } = await noir.execute(crispInputs)

  return { witness, returnValue: BigInt(returnValue as `0x${string}`) }
}

export const executeGrecoCircuit = async (grecoInputs: GrecoCircuitInputs): Promise<ExecuteCircuitResult<bigint[]>> => {
  const noir = new Noir(grecoCircuit as CompiledCircuit)

  const { witness, returnValue } = await noir.execute(grecoInputs)

  return { witness, returnValue: (returnValue as any[]).map((v) => BigInt(v as `0x${string}`)) }
}

/**
 * Generate a proof for the CRISP circuit given the circuit inputs.
 * @param circuitInputs - Inputs used in both the CRISP and GRECO circuits.
 * @returns The proof.
 */
export const generateProof = async (circuitInputs: any) => {
  const { witness: grecoWitness, returnValue: grecoReturnValue } = await executeGrecoCircuit({
    pk_commitment: circuitInputs.pk_commitment,
    ct0is: circuitInputs.ct0is,
    ct1is: circuitInputs.ct1is,
    pk0is: circuitInputs.pk0is,
    pk1is: circuitInputs.pk1is,
    r1is: circuitInputs.r1is,
    r2is: circuitInputs.r2is,
    p1is: circuitInputs.p1is,
    p2is: circuitInputs.p2is,
    u: circuitInputs.u,
    e0: circuitInputs.e0,
    e0is: circuitInputs.e0is,
    e0_quotients: circuitInputs.e0_quotients,
    e1: circuitInputs.e1,
    k1: circuitInputs.k1,
  })
  const grecoBackend = new UltraHonkBackend(
    (grecoCircuit as CompiledCircuit).bytecode,
    { threads: optimalThreadCount },
    { recursive: true },
  )

  const { proof: grecoProof, publicInputs: grecoPublicInputs } = await grecoBackend.generateProof(grecoWitness)

  const vk = await grecoBackend.getVerificationKey()
  const vkAsFields = deflattenFields(vk)
  const grecoProofAsFields = deflattenFields(grecoProof)
  const { vkHash } = await grecoBackend.generateRecursiveProofArtifacts(grecoProof, grecoPublicInputs.length)

  const { witness: crispWitness } = await executeCrispCircuit({
    prev_ct0is: circuitInputs.prev_ct0is,
    prev_ct1is: circuitInputs.prev_ct1is,
    prev_ct_commitment: circuitInputs.prev_ct_commitment,
    sum_ct0is: circuitInputs.sum_ct0is,
    sum_ct1is: circuitInputs.sum_ct1is,
    sum_r0is: circuitInputs.sum_r0is,
    sum_r1is: circuitInputs.sum_r1is,
    greco_verification_key: vkAsFields,
    greco_key_hash: vkHash,
    greco_proof: grecoProofAsFields,
    ct0is: circuitInputs.ct0is,
    ct1is: circuitInputs.ct1is,
    k1: circuitInputs.k1,
    pk_commitment: circuitInputs.pk_commitment,
    k1_commitment: grecoReturnValue[0].toString(),
    ct_commitment: grecoReturnValue[1].toString(),
    public_key_x: circuitInputs.public_key_x,
    public_key_y: circuitInputs.public_key_y,
    signature: circuitInputs.signature,
    hashed_message: circuitInputs.hashed_message,
    merkle_root: circuitInputs.merkle_root,
    merkle_proof_length: circuitInputs.merkle_proof_length,
    merkle_proof_indices: circuitInputs.merkle_proof_indices,
    merkle_proof_siblings: circuitInputs.merkle_proof_siblings,
    slot_address: circuitInputs.slot_address,
    balance: circuitInputs.balance,
    is_first_vote: circuitInputs.is_first_vote,
    is_mask_vote: circuitInputs.is_mask_vote,
    num_options: circuitInputs.num_options,
  } as CRISPCircuitInputs)

  const crispBackend = new UltraHonkBackend((crispCircuit as CompiledCircuit).bytecode, { threads: optimalThreadCount })

  const proof = await crispBackend.generateProof(crispWitness, { keccakZK: true })

  await crispBackend.destroy()
  await grecoBackend.destroy()

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
    if (vote[i] < 0) {
      throw new Error(`Invalid vote: choice ${i} is negative`)
    }
    if (vote[i] > maxValue) {
      throw new Error(`Invalid vote: choice ${i} exceeds maximum encodable value`)
    }
  }

  if (numChoices === 2) {
    // Binary: mutually exclusive
    const nonZeroCount = vote.filter((v) => v > 0).length
    if (nonZeroCount > 1) {
      throw new Error('Invalid vote: for 2 options, only one choice can be non-zero')
    }
    const votedAmount = vote.find((v) => v > 0) ?? 0
    if (votedAmount > balance) {
      throw new Error('Invalid vote: vote exceeds balance')
    }
  } else {
    // 3+ options: split allowed, total capped
    const total = vote.reduce((sum, v) => sum + v, 0)
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

  const { circuitInputs, encryptedVote } = await generateCircuitInputs({
    ...voteProofInputs,
    slotAddress: address,
    merkleProof,
    previousCiphertext: voteProofInputs.previousCiphertext,
    signature: voteProofInputs.signature,
    messageHash: voteProofInputs.messageHash,
    isMaskVote: false,
  })

  return { ...(await generateProof(circuitInputs)), encryptedVote }
}

/**
 * Generate a proof for a vote masking operation.
 * @param maskVoteProofInputs The mask vote proof inputs.
 * @returns
 */
export const generateMaskVoteProof = async (maskVoteProofInputs: MaskVoteProofInputs): Promise<ProofData> => {
  const merkleProof = generateMerkleProof(maskVoteProofInputs.balance, maskVoteProofInputs.slotAddress, maskVoteProofInputs.merkleLeaves)

  const { circuitInputs, encryptedVote } = await generateCircuitInputs({
    ...maskVoteProofInputs,
    signature: MASK_SIGNATURE,
    messageHash: SIGNATURE_MESSAGE_HASH,
    vote: getZeroVote(maskVoteProofInputs.numOptions),
    merkleProof,
    isMaskVote: true,
  })

  return { ...(await generateProof(circuitInputs)), encryptedVote }
}

/**
 * Locally verify a Noir proof.
 * @param proof - The proof to verify.
 * @returns True if the proof is valid, false otherwise.
 */
export const verifyProof = async (proof: ProofData): Promise<boolean> => {
  const backend = new UltraHonkBackend((crispCircuit as CompiledCircuit).bytecode, { threads: optimalThreadCount })

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
