// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { ZKInputsGenerator } from '@crisp-e3/zk-inputs'
import { type CircuitInputs, type IVote, MaskVoteProofInputs, VoteProofInputs } from './types'
import { toBinary } from './utils'
import { MAXIMUM_VOTE_VALUE, HALF_LARGEST_MINIMUM_DEGREE, OPTIMAL_THREAD_COUNT, FAKE_SIGNATURE, SIGNATURE_MESSAGE_HASH } from './constants'
import { extractSignatureComponents } from './signature'
import { Noir, type CompiledCircuit } from '@noir-lang/noir_js'
import { UltraHonkBackend, type ProofData } from '@aztec/bb.js'
import circuit from '../../../circuits/target/crisp_circuit.json'
import { bytesToHex, encodeAbiParameters, parseAbiParameters, numberToHex, getAddress, publicKeyToAddress } from 'viem/utils'
import { Hex, recoverPublicKey } from 'viem'

// Initialize the ZKInputsGenerator.
const zkInputsGenerator: ZKInputsGenerator = ZKInputsGenerator.withDefaults()

/**
 * Encode a vote.
 * @param vote The vote to encode.
 * @returns The encoded vote as a BigInt64Array.
 */
export const encodeVote = (vote: IVote): BigInt64Array => {
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
 * Decode an encoded tally into its decimal representation.
 * @param tally The encoded tally to decode.
 * @returns The decoded tally as an IVote.
 */
export const decodeTally = (tally: string[]): IVote => {
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
}

export const encryptVote = (vote: IVote, publicKey: Uint8Array): Uint8Array => {
  const encodedVote = encodeVote(vote)

  return zkInputsGenerator.encryptVote(publicKey, encodedVote)
}

export const generatePublicKey = (): Uint8Array => {
  return zkInputsGenerator.generatePublicKey()
}

export const generateCircuitInputs = async (proofInputs: VoteProofInputs & MaskVoteProofInputs): Promise<CircuitInputs> => {
  const encodedVote = encodeVote(proofInputs.vote)

  let crispInputs = await zkInputsGenerator.generateInputs(
    // If no previous ciphertext is provided, a placeholder ciphertext vote will be generated.
    // This is safe because the circuit will not check the ciphertext addition if
    // the previous ciphertext is not provided (is_first_vote is true).
    proofInputs.previousCiphertext || encryptVote({ yes: 0n, no: 0n }, proofInputs.publicKey),
    proofInputs.publicKey,
    encodedVote,
  )

  const { messageHash, publicKeyX, publicKeyY, signature } = await extractSignatureComponents(proofInputs.signature)

  crispInputs.hashed_message = Array.from(messageHash).map((b) => b.toString())
  crispInputs.public_key_x = Array.from(publicKeyX).map((b) => b.toString())
  crispInputs.public_key_y = Array.from(publicKeyY).map((b) => b.toString())
  crispInputs.signature = Array.from(signature).map((b) => b.toString())
  crispInputs.slot_address = proofInputs.slotAddress.toLowerCase()
  crispInputs.balance = proofInputs.balance.toString()
  crispInputs.is_first_vote = !proofInputs.previousCiphertext
  crispInputs.merkle_root = proofInputs.merkleProof.proof.root.toString()
  crispInputs.merkle_proof_length = proofInputs.merkleProof.length.toString()
  crispInputs.merkle_proof_indices = proofInputs.merkleProof.indices.map((i) => i.toString())
  crispInputs.merkle_proof_siblings = proofInputs.merkleProof.proof.siblings.map((s) => s.toString())

  return crispInputs
}

export const generateWitness = async (crispInputs: CircuitInputs): Promise<Uint8Array> => {
  const noir = new Noir(circuit as CompiledCircuit)

  const { witness } = await noir.execute(crispInputs as any)

  return witness
}

export const generateProof = async (crispInputs: CircuitInputs): Promise<ProofData> => {
  const witness = await generateWitness(crispInputs)
  const backend = new UltraHonkBackend((circuit as CompiledCircuit).bytecode, { threads: OPTIMAL_THREAD_COUNT })

  const proof = await backend.generateProof(witness, { keccakZK: true })

  await backend.destroy()

  return proof
}

export const generateVoteProof = async (voteProofInputs: VoteProofInputs) => {
  if (voteProofInputs.vote.yes > voteProofInputs.balance || voteProofInputs.vote.no > voteProofInputs.balance) {
    throw new Error('Invalid vote: vote exceeds balance')
  }

  if (voteProofInputs.vote.yes > MAXIMUM_VOTE_VALUE || voteProofInputs.vote.no > MAXIMUM_VOTE_VALUE) {
    throw new Error('Invalid vote: vote exceeds maximum allowed value')
  }

  if (voteProofInputs.vote.yes < 0n || voteProofInputs.vote.no < 0n) {
    throw new Error('Invalid vote: vote is negative')
  }

  // The address slot of an actual vote always is the address of the public key that signed the message.
  const publicKey = await recoverPublicKey({ hash: SIGNATURE_MESSAGE_HASH, signature: voteProofInputs.signature })
  const address = publicKeyToAddress(publicKey)

  const crispInputs = await generateCircuitInputs({
    ...voteProofInputs,
    slotAddress: address,
  })

  return generateProof(crispInputs)
}

export const generateMaskVoteProof = async (maskVoteProofInputs: MaskVoteProofInputs) => {
  const crispInputs = await generateCircuitInputs({
    ...maskVoteProofInputs,
    signature: FAKE_SIGNATURE,
    vote: { yes: 0n, no: 0n },
  })

  return generateProof(crispInputs)
}

export const verifyProof = async (proof: ProofData): Promise<boolean> => {
  const backend = new UltraHonkBackend((circuit as CompiledCircuit).bytecode, { threads: OPTIMAL_THREAD_COUNT })

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
