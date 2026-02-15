// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

import { getIsSlotEmpty, getPreviousCiphertext } from './state'
import { generateMaskVoteProof, generateVoteProof } from './vote'

import type { MaskVoteProofRequest, ProofData, VoteProofRequest } from './types'

/**
 * A class representing the CRISP SDK.
 */
export class CrispSDK {
  /**
   * The server URL for the CRISP SDK.
   * It's used by methods that communicate directly with the CRISP server.
   */
  private serverUrl: string

  /**
   * Create a new instance.
   * @param serverUrl
   */
  constructor(serverUrl: string) {
    this.serverUrl = serverUrl
  }

  /**
   * Generate a proof for a vote masking.
   * @param maskProofInputs - The inputs required to generate the mask vote proof.
   * @returns A promise that resolves to the generated proof data.
   */
  async generateMaskVoteProof(maskProofInputs: MaskVoteProofRequest): Promise<ProofData> {
    const isSlotEmpty = await getIsSlotEmpty(this.serverUrl, maskProofInputs.e3Id, maskProofInputs.slotAddress)

    let previousCiphertext

    if (!isSlotEmpty) {
      previousCiphertext = await getPreviousCiphertext(this.serverUrl, maskProofInputs.e3Id, maskProofInputs.slotAddress)
    }

    return generateMaskVoteProof({
      ...maskProofInputs,
      previousCiphertext,
    })
  }

  /**
   * Generate a proof for a vote.
   * @param voteProofInputs - The inputs required to generate the vote proof.
   * @returns A promise that resolves to the generated proof data.
   */
  async generateVoteProof(voteProofInputs: VoteProofRequest): Promise<ProofData> {
    const isSlotEmpty = await getIsSlotEmpty(this.serverUrl, voteProofInputs.e3Id, voteProofInputs.slotAddress)

    let previousCiphertext

    if (!isSlotEmpty) {
      previousCiphertext = await getPreviousCiphertext(this.serverUrl, voteProofInputs.e3Id, voteProofInputs.slotAddress)
    }

    return generateVoteProof({
      ...voteProofInputs,
      previousCiphertext,
    })
  }
}
