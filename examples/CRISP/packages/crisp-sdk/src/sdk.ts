import { getIsSlotEmpty, getPreviousCiphertext } from './state'
import { encryptVote, generateMaskVoteProof, generateVoteProof } from './vote'
import { ZERO_VOTE } from './constants'

import type { ProofData } from '@aztec/bb.js'
import type { MaskVoteProofRequest, VoteProofRequest } from './types'

/**
 * A class representing the Crisp SDK.
 */
export class CrispSDK {
  /**
   * The server URL for the Crisp SDK.
   * It's used by methods that communicate directly with the Crisp server.
   */
  private serverUrl: string

  /**
   * Create a new instance
   * @param serverUrl
   */
  constructor(serverUrl: string) {
    this.serverUrl = serverUrl
  }

  /**
   * Generate a proof for a vote masking.
   */
  async generateMaskVoteProof(maskProofInputs: MaskVoteProofRequest): Promise<ProofData> {
    // check if the slot is empty first
    const isSlotEmpty = await getIsSlotEmpty(this.serverUrl, maskProofInputs.e3Id, maskProofInputs.slotAddress)

    let previousCiphertext: Uint8Array
    if (!isSlotEmpty) {
      previousCiphertext = await getPreviousCiphertext(maskProofInputs.serverUrl, maskProofInputs.e3Id, maskProofInputs.slotAddress)
    } else {
      previousCiphertext = encryptVote(ZERO_VOTE, maskProofInputs.publicKey)
    }

    return generateMaskVoteProof({
      ...maskProofInputs,
      previousCiphertext,
      isFirstVote: isSlotEmpty,
    })
  }

  /**
   * Generate a proof for a vote.
   * @param voteProofInputs - The inputs required to generate the vote proof.
   * @returns A promise that resolves to the generated proof data.
   */
  async generateVoteProof(voteProofInputs: VoteProofRequest): Promise<ProofData> {
    // check if the slot is empty first
    const isSlotEmpty = await getIsSlotEmpty(this.serverUrl, voteProofInputs.e3Id, voteProofInputs.slotAddress)

    let previousCiphertext: Uint8Array
    if (!isSlotEmpty) {
      previousCiphertext = await getPreviousCiphertext(voteProofInputs.serverUrl, voteProofInputs.e3Id, voteProofInputs.slotAddress)
    } else {
      previousCiphertext = encryptVote(ZERO_VOTE, voteProofInputs.publicKey)
    }

    return generateVoteProof({
      ...voteProofInputs,
      previousCiphertext,
      isFirstVote: isSlotEmpty,
    })
  }
}
