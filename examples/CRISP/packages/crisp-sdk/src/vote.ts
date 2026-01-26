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
  decodeBytesToNumbers,
  numberArrayToBigInt64Array,
} from './utils'
import { MAXIMUM_VOTE_VALUE, MASK_SIGNATURE, ZERO_VOTE, SIGNATURE_MESSAGE_HASH } from './constants'
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
 * Encode a vote.
 * @param vote The vote to encode.
 * @returns The encoded vote as a number array.
 */
export const encodeVote = (vote: Vote): number[] => {
  const bfvParams = zkInputsGenerator.getBFVParams()
  const voteArray: number[] = []
  const length = bfvParams.degree
  const halfLength = length / 2
  const yesBinary = toBinary(vote.yes).split('')
  const noBinary = toBinary(vote.no).split('')

  // Fill first half with 'yes' binary representation (pad with leading 0s if needed)
  for (let i = 0; i < halfLength; i++) {
    const offset = halfLength - yesBinary.length
    voteArray.push(i < offset ? 0 : parseInt(yesBinary[i - offset]))
  }

  // Fill second half with 'no' binary representation (pad with leading 0s if needed)
  for (let i = 0; i < length - halfLength; i++) {
    const offset = length - halfLength - noBinary.length
    voteArray.push(i < offset ? 0 : parseInt(noBinary[i - offset]))
  }

  return voteArray
}

/**
 * Decode an encoded tally into its decimal representation.
 * @param tallyBytes The encoded tally as a hex string (bytes).
 * @returns The decoded tally as an IVote.
 */
export const decodeTally = (tallyBytes: string | number[]): Vote => {
  if (typeof tallyBytes === 'string') {
    // Convert hex string to bytes, handling both with and without 0x prefix
    // and decode to numbers.
    const hexString = tallyBytes.startsWith('0x') ? tallyBytes : `0x${tallyBytes}`
    tallyBytes = decodeBytesToNumbers(hexToBytes(hexString as Hex))
  }

  const HALF_D = tallyBytes.length / 2

  // Extract the first half for yes votes and second half for no votes
  // Votes are right-aligned with leading zeros, so we can use the entire halves
  const yesBinary = tallyBytes.slice(0, HALF_D)
  const noBinary = tallyBytes.slice(HALF_D, tallyBytes.length)

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

  return zkInputsGenerator.encryptVote(publicKey, numberArrayToBigInt64Array(encodedVote))
}

/**
 * Decrypt the vote using the secret key.
 * @param vote - The vote to decrypt.
 * @param secretKey - The secret key to use for decryption.
 * @returns The decrypted vote as a Vote.
 */
export const decryptVote = (ciphertext: Uint8Array, secretKey: Uint8Array): Vote => {
  const decryptedVote = zkInputsGenerator.decryptVote(secretKey, ciphertext)

  return decodeTally(Array.from(decryptedVote).map((v) => Number(v)))
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
export const computeCtCommitment = (ct0is: Polynomial[], ct1is: Polynomial[]): bigint => {
  const commitment = zkInputsGenerator.computeCtCommitment(
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
  const encodedVote = encodeVote(proofInputs.vote)

  let circuitInputs: any
  let encryptedVote: Uint8Array

  if (!proofInputs.previousCiphertext) {
    const result = await zkInputsGenerator.generateInputs(
      // A placeholder ciphertext vote will be generated.
      // This is safe because the circuit will not check the ciphertext addition if
      // the previous ciphertext is not provided (is_first_vote is true).
      encryptVote(ZERO_VOTE, proofInputs.publicKey),
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
  } as CRISPCircuitInputs)

  const crispBackend = new UltraHonkBackend((crispCircuit as CompiledCircuit).bytecode, { threads: optimalThreadCount })

  const proof = await crispBackend.generateProof(crispWitness, { keccakZK: true })

  await crispBackend.destroy()
  await grecoBackend.destroy()

  return proof
}

/**
 * Generate a vote proof for the CRISP circuit given the vote proof inputs.
 * @param voteProofInputs - The vote proof inputs.
 * @returns The vote proof.
 */
export const generateVoteProof = async (voteProofInputs: VoteProofInputs): Promise<ProofData> => {
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
    vote: ZERO_VOTE,
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
  const encryptedVoteCommitment = publicInputs[5] as `0x${string}`

  return encodeAbiParameters(parseAbiParameters('bytes, address, bytes32, bytes'), [
    bytesToHex(proof),
    slotAddress,
    encryptedVoteCommitment,
    bytesToHex(encryptedVote),
  ])
}
